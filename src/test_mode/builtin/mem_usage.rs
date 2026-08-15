//! 内存利用率测试模式
//!
//! 指标：RAM_Usage（%）、RAM_Total/Used/Free（GiB）、Swap_Usage（%）。
//!
//!     hw --api Test --task mem-usage --task print -- 3
//!     hw --api Test --task mem-usage --task check --filter RAM_Usage -- 3 60 10

use crate::test_mode::{Metric, ModeContext, ModeInstance, TestMode};

pub static MEM_USAGE: MemUsageMode = MemUsageMode;

pub struct MemUsageMode;

impl TestMode for MemUsageMode {
  fn name(&self) -> &'static str {
    "mem-usage"
  }
  fn description(&self) -> &'static str {
    "Memory usage: RAM used % and total/used/free GiB, swap usage"
  }
  fn create(&self) -> e_utils::AnyResult<Box<dyn ModeInstance>> {
    Ok(Box::new(MemUsage { sys: sysinfo::System::new() }))
  }
}

struct MemUsage {
  sys: sysinfo::System,
}

impl ModeInstance for MemUsage {
  fn sample(&mut self, _ctx: &ModeContext) -> e_utils::AnyResult<Vec<Metric>> {
    self.sys.refresh_memory_specifics(sysinfo::MemoryRefreshKind::nothing().with_ram().with_swap());
    let mut metrics = Vec::new();
    let total = self.sys.total_memory() as f64;
    let used = self.sys.used_memory() as f64;
    let free = self.sys.free_memory() as f64;
    if total > 0.0 {
      metrics.push(Metric::new("RAM_Usage", used / total * 100.0, "%"));
    }
    metrics.push(Metric::new("RAM_Total", crate::share::bytes_to_gib(self.sys.total_memory()), "GiB"));
    metrics.push(Metric::new("RAM_Used", crate::share::bytes_to_gib(self.sys.used_memory()), "GiB"));
    metrics.push(Metric::new("RAM_Free", crate::share::bytes_to_gib(free as u64), "GiB"));
    let swap_total = self.sys.total_swap() as f64;
    if swap_total > 0.0 {
      metrics.push(Metric::new("Swap_Usage", self.sys.used_swap() as f64 / swap_total * 100.0, "%"));
    }
    Ok(metrics)
  }
}