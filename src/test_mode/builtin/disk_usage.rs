//! 磁盘利用率 / IO 测试模式
//!
//! 指标：每个磁盘 <挂载点> Used%（占用率）、Read / Write（B/s，1s 窗口）。
//!
//!     hw --api Test --task disk-usage --task print -- 3
//!     hw --api Test --task disk-usage --task check --filter "C: Used%" -- 3 80 5

use crate::test_mode::{Metric, ModeContext, ModeInstance, TestMode};

pub static DISK_USAGE: DiskUsageMode = DiskUsageMode;

pub struct DiskUsageMode;

impl TestMode for DiskUsageMode {
  fn name(&self) -> &'static str {
    "disk-usage"
  }
  fn description(&self) -> &'static str {
    "Disk usage: per-mount used % and read/write rates (B/s)"
  }
  fn create(&self) -> e_utils::AnyResult<Box<dyn ModeInstance>> {
    Ok(Box::new(DiskUsage {
      disks: sysinfo::Disks::new_with_refreshed_list_specifics(
        sysinfo::DiskRefreshKind::nothing().with_storage().with_io_usage(),
      ),
    }))
  }
}

struct DiskUsage {
  disks: sysinfo::Disks,
}

impl ModeInstance for DiskUsage {
  fn sample(&mut self, ctx: &ModeContext) -> e_utils::AnyResult<Vec<Metric>> {
    self.disks.refresh_specifics(false, sysinfo::DiskRefreshKind::nothing().with_storage().with_io_usage());
    let mut metrics = Vec::new();
    for disk in self.disks.iter() {
      let mount = disk.mount_point().display().to_string();
      let pass = ctx.filter.is_empty() || ctx.filter.iter().any(|f| mount.contains(f.as_str()));
      if !pass {
        continue;
      }
      let total = disk.total_space() as f64;
      if total > 0.0 {
        let used_pct = (total - disk.available_space() as f64) / total * 100.0;
        metrics.push(Metric::new(format!("{} Used%", mount), used_pct, "%"));
      }
      let usage = disk.usage();
      metrics.push(Metric::new(format!("{} Read", mount), usage.read_bytes as f64, "B/s"));
      metrics.push(Metric::new(format!("{} Write", mount), usage.written_bytes as f64, "B/s"));
    }
    if metrics.is_empty() {
      return Err("No disks matched the filter".into());
    }
    Ok(metrics)
  }
}