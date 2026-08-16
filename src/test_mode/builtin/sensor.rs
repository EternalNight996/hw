//! 通用 WMI 传感器模式（CPU 主频 / 风扇转速 / 电压 / 功率，仅 Windows）
//!
//! 依赖 OHM / LHM / AIDA64 后端，规则用 `mode` + `metric`（传感器名包含匹配）过滤。
//!
//! ```text
//! hw --api Test --task cpu-clock --args print -- 3
//! hw --api Test --task fan-speed --args check --filter "Fan" -- 3 1500 500
//! hw --api Test --task voltage --args data
//! hw --api Test --task power --args print -- 3
//! ```

use crate::api_test::{HardwareType, SensorType};
use crate::test_mode::{Metric, ModeContext, ModeInstance, TestMode};

use super::wmi_backend::Backend;

/// 通用传感器模式：name/描述/传感器类型三要素
pub struct SensorMode {
  pub name: &'static str,
  pub description: &'static str,
  pub sensor_type: SensorType,
}

impl TestMode for SensorMode {
  fn name(&self) -> &'static str {
    self.name
  }
  fn description(&self) -> &'static str {
    self.description
  }
  fn create(&self) -> e_utils::AnyResult<Box<dyn ModeInstance>> {
    Ok(Box::new(Sensor {
      backend: Backend::start()?,
      sensor_type: self.sensor_type.clone(),
    }))
  }
}

struct Sensor {
  backend: Backend,
  sensor_type: SensorType,
}

impl ModeInstance for Sensor {
  fn sample(&mut self, ctx: &ModeContext) -> e_utils::AnyResult<Vec<Metric>> {
    let sensors = self.backend.query(HardwareType::ALL, self.sensor_type.clone())?;
    let unit = self.sensor_type.unit().to_string();
    let mut metrics: Vec<Metric> = sensors
      .into_iter()
      .map(|s| Metric::new(s.Name, s.Value, unit.clone()).with_range(Some(s.Min), Some(s.Max)))
      .collect();
    if !ctx.filter.is_empty() {
      metrics.retain(|m| ctx.filter.iter().any(|f| m.name.contains(f.as_str())));
    }
    if metrics.is_empty() {
      return Err(format!("无 {} 传感器", self.sensor_type).into());
    }
    Ok(metrics)
  }

  fn teardown(&mut self, _ctx: &ModeContext) -> e_utils::AnyResult<()> {
    self.backend.stop();
    Ok(())
  }
}

pub static CPU_CLOCK: SensorMode = SensorMode {
  name: "cpu-clock",
  description: "CPU clock frequency (MHz) via WMI backend (Windows)",
  sensor_type: SensorType::Clock,
};
pub static FAN_SPEED: SensorMode = SensorMode {
  name: "fan-speed",
  description: "Fan speed (RPM) via WMI backend (Windows)",
  sensor_type: SensorType::Fan,
};
pub static VOLTAGE: SensorMode = SensorMode {
  name: "voltage",
  description: "Voltage sensors (V) via WMI backend (Windows)",
  sensor_type: SensorType::Voltage,
};
pub static POWER: SensorMode = SensorMode {
  name: "power",
  description: "Power sensors (W) via WMI backend (Windows)",
  sensor_type: SensorType::Power,
};