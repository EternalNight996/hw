//! 温度测试模式（CPU / GPU / 主板，依赖 OHM / LHM / AIDA64 后端，仅 Windows）
//!
//!     hw --api Test --task temp --task print -- 3
//!     hw --api Test --task temp --task check --filter "CPU Package" -- 5 80 5

use crate::api_test::{HardwareType, SensorType};
use crate::test_mode::{Metric, ModeContext, ModeInstance, TestMode};

use super::wmi_backend::Backend;

pub static TEMP: TempMode = TempMode;

pub struct TempMode;

impl TestMode for TempMode {
  fn name(&self) -> &'static str {
    "temp"
  }
  fn description(&self) -> &'static str {
    "Temperature sensors (°C) via OHM/LHM/AIDA64 backend (Windows)"
  }
  fn create(&self) -> e_utils::AnyResult<Box<dyn ModeInstance>> {
    Ok(Box::new(Temp { backend: Backend::start()? }))
  }
}

struct Temp {
  backend: Backend,
}

impl ModeInstance for Temp {
  fn sample(&mut self, ctx: &ModeContext) -> e_utils::AnyResult<Vec<Metric>> {
    let sensors = self.backend.query(HardwareType::ALL, SensorType::Temperature)?;
    let mut metrics: Vec<Metric> = sensors
      .into_iter()
      .map(|s| Metric::new(s.Name, s.Value, "°C").with_range(Some(s.Min), Some(s.Max)))
      .collect();
    if !ctx.filter.is_empty() {
      metrics.retain(|m| ctx.filter.iter().any(|f| m.name.contains(f.as_str())));
    }
    if metrics.is_empty() {
      return Err("No temperature sensors found".into());
    }
    Ok(metrics)
  }

  fn teardown(&mut self, _ctx: &ModeContext) -> e_utils::AnyResult<()> {
    self.backend.stop();
    Ok(())
  }
}