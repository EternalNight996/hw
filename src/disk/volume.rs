//! Windows 卷枚举（WinAPI）——能读到**没有盘符 / 资源管理器里不显示**的卷
//!
//! sysinfo 只枚举有挂载点（盘符）的卷，恢复分区、OEM 分区、隐藏备份卷会漏掉。
//! 这里用 FindFirstVolumeW/FindNextVolumeW 遍历全部卷 GUID，再逐个取卷标、文件系统、
//! 挂载点与容量，因此「系统备份盘」这类不给盘符的卷也能读到已用容量。
//!
//! CLI：hw --api Disk --task usage [-- <卷标|盘符>]

use std::collections::HashMap;

use winapi::shared::minwindef::DWORD;
use winapi::um::fileapi::{
  FindFirstVolumeW, FindNextVolumeW, FindVolumeClose, GetDiskFreeSpaceExW, GetDriveTypeW, GetVolumeInformationW, GetVolumePathNamesForVolumeNameW,
};
use winapi::um::handleapi::INVALID_HANDLE_VALUE;

use super::DiskUsage;

/// 卷 GUID 路径缓冲区（形如 Volume{GUID} 的路径，远小于此）
const VOLUME_BUF: usize = 512;
/// 卷标 / 文件系统名缓冲区（MAX_PATH + 1）
const NAME_BUF: usize = 261;

fn wide(s: &str) -> Vec<u16> {
  s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn from_wide(buf: &[u16]) -> String {
  let end = buf.iter().position(|c| *c == 0).unwrap_or(buf.len());
  String::from_utf16_lossy(&buf[..end])
}

/// 卷标与文件系统（无权限等失败时返回空串，不中断枚举）
fn label_and_fs(volume: &[u16]) -> (String, String) {
  let mut label = [0u16; NAME_BUF];
  let mut fs = [0u16; NAME_BUF];
  let ok = unsafe {
    GetVolumeInformationW(
      volume.as_ptr(),
      label.as_mut_ptr(),
      label.len() as DWORD,
      std::ptr::null_mut(),
      std::ptr::null_mut(),
      std::ptr::null_mut(),
      fs.as_mut_ptr(),
      fs.len() as DWORD,
    )
  };
  if ok == 0 {
    (String::new(), String::new())
  } else {
    (from_wide(&label), from_wide(&fs))
  }
}

/// 卷的挂载点列表（无盘符的卷返回空；挂载到目录的卷返回目录路径）
fn mount_paths(volume: &[u16]) -> Vec<String> {
  let mut buf = vec![0u16; VOLUME_BUF];
  let mut need: DWORD = 0;
  let mut ok = unsafe { GetVolumePathNamesForVolumeNameW(volume.as_ptr(), buf.as_mut_ptr(), buf.len() as DWORD, &mut need) };
  if ok == 0 && need as usize > buf.len() {
    buf = vec![0u16; need as usize];
    ok = unsafe { GetVolumePathNamesForVolumeNameW(volume.as_ptr(), buf.as_mut_ptr(), buf.len() as DWORD, &mut need) };
  }
  if ok == 0 {
    return Vec::new();
  }
  // 多字符串（每个挂载点以空字符结尾，整块以空字符串收尾）
  buf.split(|c| *c == 0).filter(|s| !s.is_empty()).map(String::from_utf16_lossy).collect()
}

/// 容量：(总字节, 可用字节)；读取失败（无介质/无权限）返回 None
fn space(volume: &[u16]) -> Option<(u64, u64)> {
  let mut caller_free: u64 = 0;
  let mut total: u64 = 0;
  let mut free: u64 = 0;
  let ok = unsafe {
    GetDiskFreeSpaceExW(
      volume.as_ptr(),
      &mut caller_free as *mut u64 as *mut _,
      &mut total as *mut u64 as *mut _,
      &mut free as *mut u64 as *mut _,
    )
  };
  if ok == 0 {
    None
  } else {
    Some((total, free))
  }
}

fn drive_type(volume: &[u16]) -> String {
  match unsafe { GetDriveTypeW(volume.as_ptr()) } {
    2 => "Removable",
    3 => "Fixed",
    4 => "Remote",
    5 => "CDRom",
    6 => "RamDisk",
    _ => "Unknown",
  }
  .to_string()
}

/// 枚举所有卷（含无盘符/隐藏卷）；SSD/HDD 介质类型由 sysinfo 按盘符补充
pub fn list_volumes(disks: &sysinfo::Disks) -> e_utils::AnyResult<Vec<DiskUsage>> {
  let kinds: HashMap<String, String> = disks
    .iter()
    .map(|d| (d.mount_point().display().to_string().to_uppercase(), d.kind().to_string()))
    .collect();

  let mut buf = vec![0u16; VOLUME_BUF];
  let handle = unsafe { FindFirstVolumeW(buf.as_mut_ptr(), buf.len() as DWORD) };
  if handle == INVALID_HANDLE_VALUE {
    return Err("FindFirstVolumeW 失败（无法枚举卷）".into());
  }

  let mut out: Vec<DiskUsage> = Vec::new();
  loop {
    let guid = from_wide(&buf);
    if !guid.is_empty() {
      let volume = wide(&guid);
      let (label, fs) = label_and_fs(&volume);
      let paths = mount_paths(&volume);
      let (total, free) = space(&volume).unwrap_or((0, 0));
      let used = total.saturating_sub(free);
      let percent = if total > 0 { used as f64 / total as f64 * 100.0 } else { 0.0 };
      let mount = paths.first().cloned().unwrap_or_default();
      out.push(DiskUsage {
        kind: kinds.get(&mount.to_uppercase()).cloned().unwrap_or_else(|| "Unknown".into()),
        mount,
        label,
        fs,
        guid,
        paths,
        drive_type: drive_type(&volume),
        total_bytes: total,
        used_bytes: used,
        free_bytes: free,
        used_percent: (percent * 100.0).round() / 100.0,
        used_gib: (crate::share::bytes_to_gib(used) * 100.0).round() / 100.0,
      });
    }
    if unsafe { FindNextVolumeW(handle, buf.as_mut_ptr(), buf.len() as DWORD) } == 0 {
      break;
    }
  }
  unsafe { FindVolumeClose(handle) };

  if out.is_empty() {
    return Err("未枚举到任何卷".into());
  }
  Ok(out)
}
