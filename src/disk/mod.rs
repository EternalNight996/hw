pub async fn disk_query<T: AsRef<str>>(task: &str, args: &[T], filter: &[T]) -> e_utils::AnyResult<String> {
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
}
#[cfg(feature = "disk")]
pub use api::*;
