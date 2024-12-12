pub async fn disk_query<T: AsRef<str>>(task: &str, args: &[T], filter: &[T], is_full: bool) -> e_utils::AnyResult<String> {
  #[cfg(not(feature = "disk"))]
  return Err("Not Windows".into());
  #[cfg(feature = "disk")]
  {
    let args: Vec<&str> = args.iter().map(AsRef::as_ref).collect();
    let filter: Vec<&str> = filter.iter().map(AsRef::as_ref).collect();
    let disks = sysinfo::Disks::new_with_refreshed_list();
    for disk in disks.iter() {
      println!(
        "{} {} {} {} {} {} ",
        disk.name().to_string_lossy(),
        disk.mount_point().display(),
        disk.file_system().to_string_lossy(),
        disk.kind(),
        format!("{:.2}GB", crate::share::bytes_to_gib(disk.total_space())),
        format!("{:.2}%", disk.available_space() as f64 / disk.total_space() as f64 * 100.0),
      );
    }
    match task {
      "datas" => Ok(serde_json::to_string(&disk_datas(&disks))?),
      _ => todo!(),
    }
  }
}
#[cfg(feature = "disk")]
mod api {
  use serde::{Deserialize, Serialize};

  /// 获取所有磁盘数据
  pub fn disk_datas(slf: &sysinfo::Disks) -> Vec<DiskType> {
    slf
      .iter()
      .map(|disk| DiskType {
        name: disk.name().to_string_lossy().to_string(),
        mount_point: disk.mount_point().display().to_string(),
      })
      .collect()
  }

  #[derive(Serialize, Deserialize)]
  pub struct DiskType {
    pub name: String,
    pub mount_point: String,
  }
}
#[cfg(feature = "disk")]
pub use api::*;
