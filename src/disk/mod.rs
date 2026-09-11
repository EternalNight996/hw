pub async fn disk_query<T: AsRef<str>>(task: &str, args: &[T], filter: &[T], res: bool) -> e_utils::AnyResult<String> {
  #[cfg(not(feature = "disk"))]
  return Err("Not Windows".into());
  #[cfg(feature = "disk")]
  {
    let args: Vec<&str> = args.iter().map(AsRef::as_ref).collect();
    let filters: Vec<&str> = filter.iter().map(AsRef::as_ref).collect();
    let disks = sysinfo::Disks::new_with_refreshed_list();
    match task {
      "new-data" => Ok(serde_json::to_string(&disk_data_no_filters(&disks))?),
      "count" => Ok(serde_json::to_string(&disk_data_no_filters(&disks).len())?),
      "data" => Ok(serde_json::to_string(&disk_data(&disks, &filters))?),
      "mount-tree" => Ok(serde_json::to_string(&disk_mount_points(&disks, &filters)?)?),
      "size" => Ok(serde_json::to_string(&disk_size(&disks, &filters)?)?),
      // 各卷已用容量：--args / -- 之后的词都作为过滤词（盘符或卷标包含匹配）
      "usage" => {
        let mut terms = filters.clone();
        terms.extend(args.iter().copied());
        let list = collect_volumes(&disks, &terms);
        if list.is_empty() {
          return Err(format!("没有匹配的卷: {}", terms.join(", ")).into());
        }
        if res {
          return Ok(serde_json::to_string(&brief_volumes(&list))?);
        }
        Ok(serde_json::to_string(&list)?)
      }
      // 只取已使用容量值：单卷输出字节数，多卷空格分隔（--args / -- 之后的词都作为过滤词）
      "usage-used" => {
        let mut terms = filters.clone();
        terms.extend(args.iter().copied());
        let mut list = collect_volumes(&disks, &terms);
        if list.is_empty() {
          return Err(format!("没有匹配的卷: {}", terms.join(", ")).into());
        }
        list.sort_by(|a, b| a.mount.cmp(&b.mount).then_with(|| a.label.cmp(&b.label)).then_with(|| a.guid.cmp(&b.guid)));
        // 每项 "磁盘名 已用字节"（名称含空格时，末尾数字段即值），多项以空格分隔
        let items: Vec<String> = list.iter().map(|v| format!("{} {}", display_name(v), v.used_bytes)).collect();
        Ok(items.join(" "))
      }
      // 备份是否更新：--args <上次已用字节数>，-- 之后为过滤词；已用 > 上次 = 已更新
      "usage-check" => {
        let last = match args.first() {
          Some(v) => Some(v.trim().parse::<u64>().map_err(|_| format!("usage-check 的上次已用字节数无效: {}", v))?),
          None => None,
        };
        let (report, ok) = disk_usage_check(&disks, &filters, last);
        let json = if res {
          serde_json::to_string(&brief_usage_check(&report))?
        } else {
          serde_json::to_string(&report)?
        };
        if ok {
          Ok(json)
        } else {
          // 判定失败：内容为报告 JSON，R<...>R 的 status=false（与 etest 规则失败同一约定）
          Err(json.into())
        }
      }
      // 备份盘首件/拦截：--args <基线.json> [fast|hash] -- <卷过滤词>
      //   基线不存在 = 首件建档；存在 = 比对（丢失/改动/空盘 => status=false）
      //   不传基线 = 只输出快照摘要（平台自己存自己比）
      "backup-check" => {
        let baseline = args.first().map(|s| s.trim()).filter(|s| !s.is_empty());
        let mode = args.get(1).map(|s| s.trim());
        let (json, pass) = backup::run_backup_check(&disks, &filters, baseline, mode, res)?;
        if pass {
          Ok(json)
        } else {
          // 拦截：内容为报告 JSON，R<...>R 的 status=false
          Err(json.into())
        }
      }
      "check-load" => {
        let start = args[0].parse::<f64>()?;
        let end = args[1].parse::<f64>()?;
        Ok(serde_json::to_string(&disk_check_load(&disks, start, end)?)?)
      }
      "info" => Ok(serde_json::to_string(&disk_drive_info()?)?),
      _ => Err("Not supported".into()),
    }
  }
}
#[cfg(feature = "disk")]
mod api {
  use std::path::PathBuf;

  use e_utils::cmd::Cmd;
  pub fn disk_check_load(slf: &sysinfo::Disks, start: f64, end: f64) -> Result<Vec<(String, f64)>, String> {
    let mut results = Vec::new();
    let mut errors = Vec::new();

    for disk in slf.iter() {
      let total = disk.total_space() as f64; // 转换一次，避免重复计算
      let mount = disk.mount_point().to_string_lossy().to_string();
      let used = ((total - disk.available_space() as f64) / total * 100.0).round();

      if start <= used && used <= end {
        results.push((mount, used));
      } else {
        errors.push(format!("{} {} is not in the range of {} to {}", mount, used, start, end));
      }
    }

    if errors.is_empty() {
      Ok(results)
    } else {
      Err(errors.join(", "))
    }
  }
  /// 获取所有磁盘数据
  pub fn disk_data_no_filters(slf: &sysinfo::Disks) -> Vec<(String, String, String, String)> {
    slf
      .iter()
      .map(|disk| {
        (
          disk.mount_point().display().to_string(),
          disk.name().to_string_lossy().to_string(),
          disk.file_system().to_string_lossy().to_string(),
          disk.kind().to_string(),
        )
      })
      .collect()
  }
  /// 获取所有磁盘数据
  pub fn disk_data(slf: &sysinfo::Disks, filters: &[&str]) -> Vec<(String, String, String, String)> {
    slf
      .iter()
      .filter(|disk| filters.is_empty() || filters.iter().any(|filter| disk.mount_point().display().to_string().contains(filter)))
      .map(|disk| {
        (
          disk.mount_point().display().to_string(),
          disk.name().to_string_lossy().to_string(),
          disk.file_system().to_string_lossy().to_string(),
          disk.kind().to_string(),
        )
      })
      .collect()
  }
  /// 获取所有磁盘的根目录列表
  pub fn disk_mount_points(slf: &sysinfo::Disks, filters: &[&str]) -> e_utils::AnyResult<Vec<(PathBuf, Vec<PathBuf>)>> {
    slf
      .iter()
      .filter(|disk| filters.is_empty() || filters.iter().any(|filter| disk.mount_point().display().to_string().contains(filter)))
      .map(|disk| {
        let mount_point = disk.mount_point();
        let dirs: e_utils::AnyResult<Vec<PathBuf>> = std::fs::read_dir(mount_point)?.map(|entry| Ok(entry?.path())).collect();
        let mut dirs = dirs?;
        dirs.sort();
        Ok((mount_point.to_path_buf(), dirs))
      })
      .collect::<e_utils::AnyResult<Vec<(PathBuf, Vec<PathBuf>)>>>()
  }
  /// 获取所有磁盘的根目录列表
  pub fn disk_size(slf: &sysinfo::Disks, filters: &[&str]) -> e_utils::AnyResult<Vec<(PathBuf, String)>> {
    slf
      .iter()
      .filter(|disk| filters.is_empty() || filters.iter().any(|filter| disk.mount_point().display().to_string().contains(filter)))
      .map(|disk| {
        let mount_point = disk.mount_point();
        let size = disk.total_space() / 1024 / 1024 / 1024;
        Ok((mount_point.to_path_buf(), format!("{size}GB")))
      })
      .collect::<e_utils::AnyResult<Vec<(PathBuf, String)>>>()
  }
  /// 单个卷的使用量（已用容量是备份盘判定的主字段）
  #[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
  pub struct DiskUsage {
    /// 挂载点（如 "D:\\"）
    pub mount: String,
    /// 卷标（如 "系统备份盘"；无卷标为空串）
    pub label: String,
    /// 文件系统（NTFS / FAT32 / exFAT ...）
    pub fs: String,
    /// 磁盘类型（SSD / HDD / Unknown）
    pub kind: String,
    /// 卷 GUID 路径（WinAPI 枚举时形如 Volume{GUID}；sysinfo 路径下为空串）
    pub guid: String,
    /// 全部挂载点（无盘符的隐藏卷为空）
    pub paths: Vec<String>,
    /// 驱动器类型（Fixed / Removable / Remote / CDRom / RamDisk / Unknown）
    pub drive_type: String,
    /// 总容量（字节）
    pub total_bytes: u64,
    /// 已用容量（字节）——判定备份是否更新的依据
    pub used_bytes: u64,
    /// 可用容量（字节）
    pub free_bytes: u64,
    /// 已用占比（%，保留两位小数）
    pub used_percent: f64,
    /// 已用容量（GiB，便于阅读）
    pub used_gib: f64,
  }

  impl DiskUsage {
    /// 展示名：有卷标用 "卷标(挂载点)"，否则用挂载点
    pub fn name(&self) -> String {
      if self.label.is_empty() {
        self.mount.clone()
      } else {
        format!("{}({})", self.label, self.mount)
      }
    }
  }

  /// 磁盘展示名：卷标 > 盘符 > 卷 GUID 短名（Volume{前 8 位}）
  pub fn display_name(v: &DiskUsage) -> String {
    let label = v.label.trim();
    if !label.is_empty() {
      return label.to_string();
    }
    if !v.mount.is_empty() {
      return v.mount.clone();
    }
    if let Some(i) = v.guid.find("Volume{") {
      let short: String = v.guid[i + 7..].chars().take(8).collect();
      if !short.is_empty() {
        return format!("Volume{{{short}}}");
      }
    }
    "Unknown".to_string()
  }

  /// --res 关键字段：盘符/卷标/总量/已用/已用%
  pub fn brief_volumes(list: &[DiskUsage]) -> Vec<serde_json::Value> {
    list
      .iter()
      .map(|v| {
        serde_json::json!({
          "mount": v.mount,
          "label": v.label,
          "total_bytes": v.total_bytes,
          "used_bytes": v.used_bytes,
          "used_percent": v.used_percent,
        })
      })
      .collect()
  }

  /// --res 关键字段：判定结论 + 简明说明 + 精简卷列表
  pub fn brief_usage_check(r: &UsageCheckReport) -> serde_json::Value {
    serde_json::json!({
      "updated": r.updated,
      "last_used_bytes": r.last_used_bytes,
      "message": r.message,
      "volumes": brief_volumes(&r.volumes),
    })
  }

  /// 备份盘使用量判定报告（usage-check 输出）
  #[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
  pub struct UsageCheckReport {
    /// 是否全部匹配卷都已更新（未指定上次已用容量时恒 true，仅报告）
    pub updated: bool,
    /// 上次已用容量（字节）；None = 未指定，仅报告本次
    pub last_used_bytes: Option<u64>,
    /// 本次匹配到的卷明细
    pub volumes: Vec<DiskUsage>,
    /// 人类可读说明
    pub message: String,
  }

  /// 过滤：空 = 全部；否则盘符或卷标包含任一词（忽略大小写与空白词）
  pub fn usage_filter_match(filters: &[&str], mount: &str, label: &str) -> bool {
    let terms: Vec<String> = filters.iter().map(|f| f.trim().to_lowercase()).filter(|f| !f.is_empty()).collect();
    if terms.is_empty() {
      return true;
    }
    let mount = mount.to_lowercase();
    let label = label.to_lowercase();
    terms.iter().any(|f| mount.contains(f.as_str()) || label.contains(f.as_str()))
  }

  /// 卷是否命中过滤词（盘符 / 卷标 / 挂载点 / 卷 GUID 任一包含）
  pub fn usage_matches(filters: &[&str], v: &DiskUsage) -> bool {
    usage_filter_match(filters, &v.mount, &v.label)
      || usage_filter_match(filters, &v.guid, "")
      || v.paths.iter().any(|p| usage_filter_match(filters, p, ""))
  }

  /// 枚举卷：Windows 走 WinAPI（含无盘符/隐藏卷），其他平台走 sysinfo
  pub fn collect_volumes(slf: &sysinfo::Disks, filters: &[&str]) -> Vec<DiskUsage> {
    #[cfg(target_os = "windows")]
    {
      if let Ok(list) = super::volume::list_volumes(slf) {
        return list.into_iter().filter(|v| usage_matches(filters, v)).collect();
      }
    }
    disk_usage_list(slf, filters)
  }

  /// 列举匹配卷的已用容量
  pub fn disk_usage_list(slf: &sysinfo::Disks, filters: &[&str]) -> Vec<DiskUsage> {
    slf
      .iter()
      .filter(|disk| usage_filter_match(filters, &disk.mount_point().display().to_string(), &disk.name().to_string_lossy()))
      .map(|disk| {
        let total = disk.total_space();
        let free = disk.available_space();
        let used = total.saturating_sub(free);
        let percent = if total > 0 { used as f64 / total as f64 * 100.0 } else { 0.0 };
        let mount = disk.mount_point().display().to_string();
        DiskUsage {
          mount: mount.clone(),
          label: disk.name().to_string_lossy().to_string(),
          fs: disk.file_system().to_string_lossy().to_string(),
          kind: disk.kind().to_string(),
          guid: String::new(),
          paths: vec![mount],
          drive_type: String::new(),
          total_bytes: total,
          used_bytes: used,
          free_bytes: free,
          used_percent: (percent * 100.0).round() / 100.0,
          used_gib: (crate::share::bytes_to_gib(used) * 100.0).round() / 100.0,
        }
      })
      .collect()
  }

  /// 按“本次已用 > 上次已用 = 已更新”判定；返回（报告, 是否通过）
  pub fn usage_check_report(volumes: Vec<DiskUsage>, last: Option<u64>) -> (UsageCheckReport, bool) {
    if volumes.is_empty() {
      return (
        UsageCheckReport {
          updated: false,
          last_used_bytes: last,
          volumes,
          message: "没有匹配的卷（检查卷标或盘符过滤词）".into(),
        },
        false,
      );
    }
    let detail = |last: u64| {
      volumes
        .iter()
        .map(|v| {
          format!(
            "{} 已用 {} 字节{}",
            v.name(),
            v.used_bytes,
            if v.used_bytes > last { "（已更新）" } else { "（未更新）" }
          )
        })
        .collect::<Vec<_>>()
        .join("; ")
    };
    match last {
      Some(last) => {
        let updated = volumes.iter().all(|v| v.used_bytes > last);
        let message = format!("上次已用 {} 字节；{}", last, detail(last));
        (
          UsageCheckReport {
            updated,
            last_used_bytes: Some(last),
            volumes,
            message,
          },
          updated,
        )
      }
      None => {
        let message = volumes
          .iter()
          .map(|v| format!("{} 已用 {:.2} GiB（{} 字节，{:.2}%）", v.name(), v.used_gib, v.used_bytes, v.used_percent))
          .collect::<Vec<_>>()
          .join("; ");
        (
          UsageCheckReport {
            updated: true,
            last_used_bytes: None,
            volumes,
            message,
          },
          true,
        )
      }
    }
  }

  /// 从磁盘列表直接判定（usage-check 用）
  pub fn disk_usage_check(slf: &sysinfo::Disks, filters: &[&str], last: Option<u64>) -> (UsageCheckReport, bool) {
    usage_check_report(collect_volumes(slf, filters), last)
  }

  pub fn disk_drive_info() -> e_utils::Result<Vec<DiskInfo>> {
    // 执行 WMIC 命令（优化命令参数）
    let output = Cmd::new("wmic").args(&["diskdrive", "get", "Model,Name,PNPDeviceID", "/format:csv"]).output()?;

    // 解析 CSV 输出（带容错处理）
    let disks: Vec<DiskInfo> = output
      .stdout
      .lines()
      .skip(1) // 跳过 CSV 表头（例如：Node,Model,Name,PNPDeviceID）
      .filter_map(|line| {
        let cleaned = line.trim().replace('\r', "");
        if cleaned.is_empty() {
          return None;
        }

        // 分割字段并校验完整性
        let fields: Vec<&str> = cleaned.split(',').collect();
        if fields.len() >= 4 {
          // Node,Model,Name,PNPDeviceID
          Some(DiskInfo {
            model: fields[1].trim().to_string(),
            name: fields[2].trim().to_string(),
            pnp_device_id: fields[3].trim().to_string(),
          })
        } else {
          None // 自动过滤无效数据行
        }
      })
      .collect();

    Ok(disks)
  }

  #[derive(Debug, PartialEq, Clone, serde::Deserialize, serde::Serialize)]
  pub struct DiskInfo {
    pub model: String,
    pub name: String,
    pub pnp_device_id: String,
  }

  #[cfg(test)]
  mod tests {
    use super::*;

    fn vol(label: &str, mount: &str, used: u64) -> DiskUsage {
      DiskUsage {
        mount: mount.into(),
        label: label.into(),
        fs: "NTFS".into(),
        kind: "HDD".into(),
        guid: format!("Volume{{{}}}", mount),
        paths: vec![mount.into()],
        drive_type: "Fixed".into(),
        total_bytes: 1000,
        used_bytes: used,
        free_bytes: 1000 - used,
        used_percent: used as f64 / 10.0,
        used_gib: used as f64 / 1073741824.0,
      }
    }

    #[test]
    fn display_name_prefers_label_then_mount_then_guid() {
      let mut v = vol("系统备份盘", "G:\\", 100);
      assert_eq!(display_name(&v), "系统备份盘");
      v.label = "  ".into();
      assert_eq!(display_name(&v), "G:\\");
      v.mount = String::new();
      v.guid = "Volume{4514bc2d-5123-4f5e-a737-6ebf5e7380bf}".into();
      assert_eq!(display_name(&v), "Volume{4514bc2d}");
      v.guid = String::new();
      assert_eq!(display_name(&v), "Unknown");
    }

    #[test]
    fn filter_by_label_or_mount() {
      assert!(usage_filter_match(&[], "D:\\", "系统备份盘")); // 空 = 全部
      assert!(usage_filter_match(&["   "], "D:\\", "x")); // 空白词忽略
      assert!(usage_filter_match(&["系统备份盘"], "D:\\", "系统备份盘")); // 卷标
      assert!(usage_filter_match(&["d:"], "D:\\", "")); // 盘符（忽略大小写）
      assert!(!usage_filter_match(&["系统备份盘"], "D:\\", "软件"));
      assert!(!usage_filter_match(&["Z:"], "D:\\", "软件"));
    }

    #[test]
    fn matches_volume_guid_and_paths() {
      // 无盘符卷：靠卷标 / 卷 GUID 命中
      let v = vol("系统备份盘", "", 800);
      assert!(usage_matches(&["系统备份盘"], &v));
      assert!(usage_matches(&["volume{"], &v));
      assert!(!usage_matches(&["Z:"], &v));
      assert!(usage_matches(&["d:"], &vol("", "D:\\", 10)));
    }

    #[test]
    fn check_updated_when_used_grows() {
      let list = vec![vol("系统备份盘", "G:\\", 800)];
      let (report, ok) = usage_check_report(list.clone(), Some(700));
      assert!(ok && report.updated);
      assert!(report.message.contains("已更新"));
      // 未增长 = 未更新
      let (report, ok) = usage_check_report(list.clone(), Some(800));
      assert!(!ok && !report.updated);
      assert!(report.message.contains("未更新"));
    }

    #[test]
    fn check_without_last_only_reports() {
      let (report, ok) = usage_check_report(vec![vol("系统备份盘", "G:\\", 800)], None);
      assert!(ok && report.last_used_bytes.is_none());
      assert_eq!(report.volumes[0].used_bytes, 800);
    }

    #[test]
    fn check_all_volumes_must_grow() {
      let list = vec![vol("备份A", "G:\\", 800), vol("备份B", "H:\\", 500)];
      let (report, ok) = usage_check_report(list, Some(600));
      assert!(!ok && !report.updated); // B 未增长 -> 整体未通过
      assert_eq!(report.volumes.len(), 2);
    }

    #[test]
    fn check_no_match_fails_structured() {
      let (report, ok) = usage_check_report(Vec::new(), Some(1));
      assert!(!ok);
      assert!(report.volumes.is_empty());
      assert!(serde_json::to_string(&report).unwrap().contains("没有匹配的卷"));
    }
  }
}
#[cfg(feature = "disk")]
pub use api::*;
// Windows 专用：WinAPI 全卷枚举（含无盘符/隐藏卷，sysinfo 枚举不到）
#[cfg(all(feature = "disk", target_os = "windows"))]
mod volume;
#[cfg(all(feature = "disk", target_os = "windows"))]
pub use volume::*;
// 备份盘首件基线 / 后续拦截（扫描 + 指纹 + 差异）
#[cfg(feature = "disk")]
pub mod backup;
