//! 备份盘「首件基线 → 后续拦截」：把备份情况快照成可持久化指纹，再逐次比对
//!
//! 一、首件：基线文件不存在 → 全盘扫描并写入基线 JSON（含完整清单），R 状态 true
//! 二、后续：基线文件存在 → 重新扫描并比对，缺文件/被改动/空盘 → R 状态 false（平台拦截）
//! 三、不传基线路径 → 只输出快照摘要（平台自己存自己比）
//!
//! 默认快模式：指纹 = 全部 (相对路径, 大小, mtime) 取 SHA-256，25G 盘秒级；
//! hash 模式额外对每个文件内容做 SHA-256（可靠但慢，适合离线深度校验）。
//!
//! 无盘符/资源管理器隐藏的卷也能扫（走卷 GUID 路径），无权限目录计入 skipped 不算失败。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{collect_volumes, DiskUsage};

/// 无权限/系统目录：遍历时跳过并计入 skipped
const SKIP_DIRS: [&str; 2] = ["System Volume Information", "$RECYCLE.BIN"];
/// 快照里保留的最大文件条目数（人工定位用）
const LARGEST_KEEP: usize = 10;
/// 差异报告里最多列出的样例条数
const DIFF_KEEP: usize = 15;

/// 单个文件条目（指纹的组成单元）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEntry {
  /// 相对卷根的路径（统一用 / 分隔）
  pub path: String,
  pub size: u64,
  /// 修改时间（Unix 秒）
  pub mtime: u64,
}

/// 快照摘要（进 R 行输出，保持小而可读）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupSummary {
  /// 卷展示名（卷标(挂载点) 或 GUID）
  pub volume: String,
  /// 扫描根路径（盘符或卷 GUID 路径）
  pub root: String,
  /// 采集时间（Unix 秒）
  pub captured_at: u64,
  pub dirs: usize,
  pub files: usize,
  pub total_bytes: u64,
  /// 最新文件时间（Unix 秒）
  pub latest_mtime: u64,
  /// 因无权限/系统目录/链接跳过的条目数
  pub skipped: usize,
  /// 结构指纹：sha256:...
  pub fingerprint: String,
  /// 最大的若干文件（人工定位用）
  pub largest: Vec<FileEntry>,
}

/// 首件基线（落盘 JSON；entries 为完整清单，用于后续差异定位）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupSnapshot {
  #[serde(flatten)]
  pub summary: BackupSummary,
  /// 完整文件清单（按路径排序）
  pub entries: Vec<FileEntry>,
  /// hash 模式下每个文件的 sha256（快模式为空）
  #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
  pub hashes: BTreeMap<String, String>,
}

/// 差异统计（added 放行，removed/changed 拦截）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupDiff {
  pub added: usize,
  pub removed: usize,
  pub changed: usize,
  /// 差异样例（最多 DIFF_KEEP 条）
  pub samples: Vec<String>,
}

/// 备份校验报告（backup-check 输出；pass=false 时 R 状态也为 false）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupReport {
  /// snapshot（只报告）/ init（首件建档）/ verify（比对拦截）
  pub mode: String,
  pub pass: bool,
  pub fingerprint_before: Option<String>,
  pub fingerprint_now: String,
  pub files_before: Option<usize>,
  pub files_now: usize,
  pub diff: BackupDiff,
  pub message: String,
  /// 本次快照摘要（平台可用它滚动更新基线）
  pub summary: BackupSummary,
}

fn now_secs() -> u64 {
  SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn to_hex(bytes: &[u8]) -> String {
  let mut s = String::with_capacity(bytes.len() * 2);
  for b in bytes {
    s.push_str(&format!("{b:02x}"));
  }
  s
}

/// 文件内容 SHA-256（流式 1MiB 分块，大镜像文件不占内存）
fn file_sha256(path: &Path) -> Option<String> {
  use std::io::Read as _;
  let mut f = std::fs::File::open(path).ok()?;
  let mut hasher = Sha256::new();
  let mut buf = vec![0u8; 1024 * 1024];
  loop {
    let n = f.read(&mut buf).ok()?;
    if n == 0 {
      break;
    }
    hasher.update(&buf[..n]);
  }
  Some(format!("sha256:{}", to_hex(&hasher.finalize())))
}

/// 结构指纹：路径 + 大小 + mtime（hash 模式并入内容摘要），与遍历顺序无关
pub fn fingerprint(entries: &[FileEntry], hashes: &BTreeMap<String, String>) -> String {
  let mut sorted: Vec<&FileEntry> = entries.iter().collect();
  sorted.sort_by(|a, b| a.path.cmp(&b.path));
  let mut hasher = Sha256::new();
  for e in sorted {
    hasher.update(e.path.as_bytes());
    hasher.update([0u8]);
    hasher.update(e.size.to_le_bytes());
    hasher.update(e.mtime.to_le_bytes());
    if let Some(h) = hashes.get(&e.path) {
      hasher.update(h.as_bytes());
    }
    hasher.update([0u8]);
  }
  format!("sha256:{}", to_hex(&hasher.finalize()))
}

/// 扫描结果
#[derive(Debug, Clone, Default)]
pub struct Scan {
  pub entries: Vec<FileEntry>,
  pub hashes: BTreeMap<String, String>,
  pub dirs: usize,
  pub skipped: usize,
  pub total_bytes: u64,
  pub latest_mtime: u64,
}

/// 递归扫描卷；无权限目录跳过并计入 skipped（不中断、不失败）
pub fn scan(root: &Path, want_hash: bool) -> e_utils::AnyResult<Scan> {
  let mut out = Scan::default();
  let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
  while let Some(dir) = stack.pop() {
    let rd = match std::fs::read_dir(&dir) {
      Ok(rd) => rd,
      Err(_) => {
        // 无权限（如 System Volume Information）或中途拔盘：跳过，不算失败
        out.skipped += 1;
        continue;
      }
    };
    for entry in rd.flatten() {
      let path = entry.path();
      let ft = match entry.file_type() {
        Ok(ft) => ft,
        Err(_) => {
          out.skipped += 1;
          continue;
        }
      };
      // 符号链接/重解析点不跟随，避免环路与重复计数
      if ft.is_symlink() {
        out.skipped += 1;
        continue;
      }
      if ft.is_dir() {
        let name = entry.file_name().to_string_lossy().to_string();
        if SKIP_DIRS.iter().any(|s| s.eq_ignore_ascii_case(&name)) {
          out.skipped += 1;
          continue;
        }
        out.dirs += 1;
        stack.push(path);
        continue;
      }
      if !ft.is_file() {
        out.skipped += 1;
        continue;
      }
      let meta = match entry.metadata() {
        Ok(m) => m,
        Err(_) => {
          out.skipped += 1;
          continue;
        }
      };
      let rel = path
        .strip_prefix(root)
        .unwrap_or(&path)
        .to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/");
      let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
      let size = meta.len();
      if want_hash {
        match file_sha256(&path) {
          Some(h) => {
            out.hashes.insert(rel.clone(), h);
          }
          None => out.skipped += 1,
        }
      }
      out.total_bytes += size;
      out.latest_mtime = out.latest_mtime.max(mtime);
      out.entries.push(FileEntry { path: rel, size, mtime });
    }
  }
  out.entries.sort_by(|a, b| a.path.cmp(&b.path));
  Ok(out)
}

/// 由扫描结果构造快照（摘要 + 完整清单）
pub fn snapshot_of(vol: &DiskUsage, root: &Path, scan: &Scan) -> BackupSnapshot {
  let mut largest: Vec<FileEntry> = scan.entries.clone();
  largest.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.path.cmp(&b.path)));
  largest.truncate(LARGEST_KEEP);
  BackupSnapshot {
    summary: BackupSummary {
      volume: vol.name(),
      root: root.display().to_string(),
      captured_at: now_secs(),
      dirs: scan.dirs,
      files: scan.entries.len(),
      total_bytes: scan.total_bytes,
      latest_mtime: scan.latest_mtime,
      skipped: scan.skipped,
      fingerprint: fingerprint(&scan.entries, &scan.hashes),
      largest,
    },
    entries: scan.entries.clone(),
    hashes: scan.hashes.clone(),
  }
}

/// 比对基线与本次快照：added 放行（备份递增正常），removed/changed 视为异常
pub fn diff(before: &BackupSnapshot, now: &BackupSnapshot) -> BackupDiff {
  let old: BTreeMap<&str, &FileEntry> = before.entries.iter().map(|e| (e.path.as_str(), e)).collect();
  let new: BTreeMap<&str, &FileEntry> = now.entries.iter().map(|e| (e.path.as_str(), e)).collect();
  let mut d = BackupDiff {
    added: 0,
    removed: 0,
    changed: 0,
    samples: Vec::new(),
  };
  for (path, b) in &old {
    match new.get(path) {
      None => {
        d.removed += 1;
        if d.samples.len() < DIFF_KEEP {
          d.samples.push(format!("丢失 {} ({} 字节)", path, b.size));
        }
      }
      Some(n) => {
        let meta_changed = n.size != b.size || n.mtime != b.mtime;
        // hash 模式下，元数据相同但内容不同也算改动
        let hash_changed = match (before.hashes.get(*path), now.hashes.get(*path)) {
          (Some(x), Some(y)) => x != y,
          _ => false,
        };
        if meta_changed || hash_changed {
          d.changed += 1;
          if d.samples.len() < DIFF_KEEP {
            if hash_changed && !meta_changed {
              d.samples.push(format!("改动 {} ({} 字节, 内容不同)", path, n.size));
            } else {
              d.samples.push(format!("改动 {} ({} -> {} 字节)", path, b.size, n.size));
            }
          }
        }
      }
    }
  }
  for path in new.keys() {
    if !old.contains_key(path) {
      d.added += 1;
      if d.samples.len() < DIFF_KEEP {
        d.samples.push(format!("新增 {}", path));
      }
    }
  }
  d
}

/// --res 关键字段：判定结论 + 差异计数 + 少量样例（完整报告仍在日志里）
pub fn brief_report(r: &BackupReport) -> serde_json::Value {
  serde_json::json!({
    "mode": r.mode,
    "pass": r.pass,
    "message": r.message,
    "diff": {
      "added": r.diff.added,
      "removed": r.diff.removed,
      "changed": r.diff.changed,
      "samples": r.diff.samples.iter().take(5).collect::<Vec<_>>(),
    },
    "files_before": r.files_before,
    "files_now": r.files_now,
    "fingerprint_now": r.fingerprint_now,
  })
}

/// 备份校验入口：返回（报告 JSON, 是否通过）；res=true 时只输出关键字段
pub fn run_backup_check(
  disks: &sysinfo::Disks,
  filters: &[&str],
  baseline: Option<&str>,
  mode: Option<&str>,
  res: bool,
) -> e_utils::AnyResult<(String, bool)> {
  let volumes: Vec<DiskUsage> = collect_volumes(disks, filters);
  if volumes.is_empty() {
    return Err(format!("没有匹配的备份卷: {}", filters.join(", ")).into());
  }
  if volumes.len() > 1 {
    let names: Vec<String> = volumes.iter().map(|v| v.name()).collect();
    return Err(format!("匹配到 {} 个卷，请用卷标/盘符/GUID 唯一定位: {}", volumes.len(), names.join(", ")).into());
  }
  let vol = &volumes[0];
  let root = root_of(vol)?;
  let want_hash = mode.map(|m| m.eq_ignore_ascii_case("hash")).unwrap_or(false);
  let scan = scan(&root, want_hash)?;
  let snap = snapshot_of(vol, &root, &scan);

  let report = match baseline {
    None => BackupReport {
      mode: "snapshot".into(),
      pass: true,
      fingerprint_before: None,
      fingerprint_now: snap.summary.fingerprint.clone(),
      files_before: None,
      files_now: snap.summary.files,
      diff: BackupDiff {
        added: 0,
        removed: 0,
        changed: 0,
        samples: Vec::new(),
      },
      message: format!("快照完成: {} 个文件 / {} 字节", snap.summary.files, snap.summary.total_bytes),
      summary: snap.summary,
    },
    Some(path) => {
      let file = Path::new(path);
      if file.exists() {
        let before: BackupSnapshot = serde_json::from_str(&std::fs::read_to_string(file)?)?;
        let d = diff(&before, &snap);
        let pass = d.removed == 0 && d.changed == 0 && snap.summary.files > 0;
        let message = if snap.summary.files == 0 {
          "备份盘为空（没有备份内容）".to_string()
        } else if pass {
          format!("备份完好: {} 个文件, 新增 {} 个", snap.summary.files, d.added)
        } else {
          format!("备份异常: 丢失 {} 个, 改动 {} 个, 新增 {} 个", d.removed, d.changed, d.added)
        };
        BackupReport {
          mode: "verify".into(),
          pass,
          fingerprint_before: Some(before.summary.fingerprint.clone()),
          fingerprint_now: snap.summary.fingerprint.clone(),
          files_before: Some(before.summary.files),
          files_now: snap.summary.files,
          diff: d,
          message,
          summary: snap.summary,
        }
      } else {
        std::fs::write(file, serde_json::to_string_pretty(&snap)?)?;
        BackupReport {
          mode: "init".into(),
          pass: true,
          fingerprint_before: None,
          fingerprint_now: snap.summary.fingerprint.clone(),
          files_before: None,
          files_now: snap.summary.files,
          diff: BackupDiff {
            added: 0,
            removed: 0,
            changed: 0,
            samples: Vec::new(),
          },
          message: format!("首件基线已建立: {} ({} 个文件)", path, snap.summary.files),
          summary: snap.summary,
        }
      }
    }
  };
  let pass = report.pass;
  let json = if res {
    serde_json::to_string(&brief_report(&report))?
  } else {
    serde_json::to_string(&report)?
  };
  Ok((json, pass))
}

/// 卷的扫描根：优先盘符挂载点，其次卷 GUID 路径（无盘符/隐藏卷）
fn root_of(vol: &DiskUsage) -> e_utils::AnyResult<PathBuf> {
  if let Some(p) = vol.paths.first() {
    return Ok(PathBuf::from(p));
  }
  if !vol.guid.is_empty() {
    return Ok(PathBuf::from(&vol.guid));
  }
  Err(format!("卷 {} 没有可访问的挂载点", vol.name()).into())
}

#[cfg(test)]
mod tests {
  use super::*;

  fn vol_paths(label: &str, mount: &str, used: u64) -> DiskUsage {
    DiskUsage {
      mount: mount.into(),
      label: label.into(),
      fs: "NTFS".into(),
      kind: "HDD".into(),
      guid: String::new(),
      paths: vec![mount.into()],
      drive_type: "Fixed".into(),
      total_bytes: 1000,
      used_bytes: used,
      free_bytes: 1000 - used,
      used_percent: 0.0,
      used_gib: 0.0,
    }
  }

  fn e(path: &str, size: u64, mtime: u64) -> FileEntry {
    FileEntry {
      path: path.into(),
      size,
      mtime,
    }
  }

  fn scan_of(items: &[(&str, u64, u64)]) -> Scan {
    Scan {
      entries: items.iter().map(|(p, s, m)| e(p, *s, *m)).collect(),
      ..Default::default()
    }
  }

  #[test]
  fn fingerprint_is_order_independent() {
    let a = vec![e("b.bin", 2, 20), e("a.bin", 1, 10)];
    let b = vec![e("a.bin", 1, 10), e("b.bin", 2, 20)];
    let empty = BTreeMap::new();
    assert_eq!(fingerprint(&a, &empty), fingerprint(&b, &empty));
    let c = vec![e("a.bin", 9, 10), e("b.bin", 2, 20)];
    assert_ne!(fingerprint(&a, &empty), fingerprint(&c, &empty));
  }

  #[test]
  fn diff_classifies_added_removed_changed() {
    let v = vol_paths("系统备份盘", "G:", 800);
    let before = snapshot_of(&v, Path::new("G:"), &scan_of(&[("a.bin", 1, 10), ("b.bin", 2, 20)]));
    let now = snapshot_of(&v, Path::new("G:"), &scan_of(&[("a.bin", 5, 10), ("c.bin", 3, 30)]));
    let d = diff(&before, &now);
    assert_eq!((d.removed, d.changed, d.added), (1, 1, 1));
    assert_eq!(d.samples.len(), 3);
  }

  #[test]
  fn diff_clean_when_identical() {
    let v = vol_paths("系统备份盘", "G:", 800);
    let s = scan_of(&[("a.bin", 1, 10)]);
    let before = snapshot_of(&v, Path::new("G:"), &s);
    let after = snapshot_of(&v, Path::new("G:"), &s);
    let d = diff(&before, &after);
    assert_eq!((d.removed, d.changed, d.added), (0, 0, 0));
    assert_eq!(before.summary.fingerprint, after.summary.fingerprint);
  }

  #[test]
  fn diff_detects_same_size_content_change_via_hash() {
    let v = vol_paths("系统备份盘", "G:", 800);
    let s = scan_of(&[("a.bin", 100, 10)]);
    let mut before = snapshot_of(&v, Path::new("G:"), &s);
    let mut now = snapshot_of(&v, Path::new("G:"), &s);
    before.hashes.insert("a.bin".into(), "sha256:cafebabe".into());
    now.hashes.insert("a.bin".into(), "sha256:deadbeef".into());
    let d = diff(&before, &now);
    assert_eq!(d.changed, 1); // 大小/mtime 相同，内容不同
  }

  #[test]
  fn snapshot_roundtrip_via_json() {
    let v = vol_paths("系统备份盘", "", 800);
    let s = scan_of(&[("dir/a.bin", 7, 70), ("b.bin", 1, 10)]);
    let snap = snapshot_of(&v, Path::new("Volume{test}"), &s);
    let text = serde_json::to_string_pretty(&snap).unwrap();
    let back: BackupSnapshot = serde_json::from_str(&text).unwrap();
    assert_eq!(snap.summary.files, 2);
    assert_eq!(back.entries.len(), 2);
    assert_eq!(back.summary.largest[0].path, "dir/a.bin");
    assert_eq!(snap.summary.fingerprint, back.summary.fingerprint);
  }
}
