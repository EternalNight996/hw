//! CPU 利用率/频率测试模式
//!
//! 指标：CPU_Usage_Global（%）；--full 或 --filter 时输出每核心利用率与主频。
//! check 模式支持 v3 负载（0-100%），复用 LoadController 忙等生成器。
//!
//! ```text
//! hw --api Test --task cpu-usage --args check --filter CPU_Usage_Global -- 5 80 10 100
//! ```

use crate::api_test::{HardwareType, SensorType, LOAD_CONTROLLER};
use crate::test_mode::{Metric, ModeContext, ModeInstance, TestMode};

pub static CPU_USAGE: CpuUsageMode = CpuUsageMode;

pub struct CpuUsageMode;

impl TestMode for CpuUsageMode {
  fn name(&self) -> &'static str {
    "cpu-usage"
  }
  fn description(&self) -> &'static str {
    "CPU usage (%) and per-core clock (MHz); check supports load generation"
  }
  fn create(&self) -> e_utils::AnyResult<Box<dyn ModeInstance>> {
    Ok(Box::new(CpuUsage { sys: sysinfo::System::new() }))
  }
}

struct CpuUsage {
  sys: sysinfo::System,
}

impl ModeInstance for CpuUsage {
  fn sample(&mut self, ctx: &ModeContext) -> e_utils::AnyResult<Vec<Metric>> {
    let need_detail = ctx.is_full || !ctx.filter.is_empty();
    let with_clock = ctx.is_full || ctx.filter.iter().any(|f| {
      let f = f.to_ascii_lowercase();
      ["clock", "freq", "mhz"].iter().any(|k| f.contains(k))
    });
    let mut refresh = sysinfo::CpuRefreshKind::nothing().with_cpu_usage();
    if with_clock {
      refresh = refresh.with_frequency();
    }
    self.sys.refresh_cpu_specifics(refresh);
    let mut metrics = vec![Metric::new("CPU_Usage_Global", self.sys.global_cpu_usage() as f64, "%")];
    if need_detail {
      for (i, cpu) in self.sys.cpus().iter().enumerate() {
        metrics.push(Metric::new(format!("CPU_{}_Usage", i), cpu.cpu_usage() as f64, "%"));
        if with_clock {
          metrics.push(Metric::new(format!("CPU_{}_Clock", i), cpu.frequency() as f64, "MHz"));
        }
      }
    }
    Ok(metrics)
  }

  fn spawn_load(&self, _ctx: &ModeContext, target: f64) -> e_utils::AnyResult<Vec<std::thread::JoinHandle<()>>> {
    let core_count = self.sys.cpus().len().max(1);
    LOAD_CONTROLLER.set_loaded(target);
    Ok(crate::api_test::LoadController::spawn_load(core_count, &HardwareType::CPU, &SensorType::Load, target)?)
  }
}