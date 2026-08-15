//! GPU 利用率测试模式（依赖 OHM / LHM / AIDA64 后端，仅 Windows）
//!
//! ```text
//! hw --api Test --task gpu-usage --args print -- 3
//! hw --api Test --task gpu-usage --args check --filter "GPU Core" -- 5 90 10
//! ```

use crate::api_test::{HardwareType, SensorType};
use crate::test_mode::{Metric, ModeContext, ModeInstance, TestMode};

use super::wmi_backend::Backend;

pub static GPU_USAGE: GpuUsageMode = GpuUsageMode;

pub struct GpuUsageMode;

impl TestMode for GpuUsageMode {
  fn name(&self) -> &'static str {
    "gpu-usage"
  }
  fn description(&self) -> &'static str {
    "GPU utilization (%) via OHM/LHM/AIDA64 backend (Windows)"
  }
  fn create(&self) -> e_utils::AnyResult<Box<dyn ModeInstance>> {
    Ok(Box::new(Gpu { backend: Backend::start()? }))
  }
}

struct Gpu {
  backend: Backend,
}

impl ModeInstance for Gpu {
  fn sample(&mut self, ctx: &ModeContext) -> e_utils::AnyResult<Vec<Metric>> {
    // NVIDIA 与 AMD 分开查询；空结果视为无对应显卡，继续
    let mut sensors = self
      .backend
      .query(HardwareType::GpuNvidia, SensorType::Load)
      .unwrap_or_default()
      .into_iter()
      .chain(
        self.backend.query(HardwareType::GpuAti, SensorType::Load).unwrap_or_default().into_iter(),
      )
      .collect::<Vec<_>>();
    if sensors.is_empty() {
      // 兜底：全量 Load 传感器中名字含 GPU 的
      sensors = self
        .backend
        .query(HardwareType::ALL, SensorType::Load)
        .unwrap_or_default()
        .into_iter()
        .filter(|s| s.Name.contains("GPU") || s.Name.contains("Graphics"))
        .collect();
    }
    let mut metrics: Vec<Metric> = sensors
      .into_iter()
      .map(|s| Metric::new(s.Name, s.Value, "%").with_range(Some(s.Min), Some(s.Max)))
      .collect();
    if !ctx.filter.is_empty() {
      metrics.retain(|m| ctx.filter.iter().any(|f| m.name.contains(f.as_str())));
    }
    if metrics.is_empty() {
      return Err("No GPU load sensors found".into());
    }
    Ok(metrics)
  }

  fn teardown(&mut self, _ctx: &ModeContext) -> e_utils::AnyResult<()> {
    self.backend.stop();
    Ok(())
  }
}