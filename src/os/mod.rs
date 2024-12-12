#[allow(unused)]
use crate::api_test::{HardwareType, Sensor, SensorType};
use crate::share::bytes_to_gib;
pub use sysinfo::*;

/// OS
#[derive(Debug)]
pub struct OS(System);
impl OS {
  pub fn new() -> Self {
    Self(System::new())
  }
  /// 获取系统
  pub fn get_mut(&mut self) -> &mut System {
    &mut self.0
  }
  pub fn get(&self) -> &System {
    &self.0
  }
}

/// CPU
impl OS {
  /// 获取cpu核心数
  pub fn get_cpu_core_count(&self) -> usize {
    self.0.cpus().len()
  }
}
/// 接口
impl OS {
  pub fn query(&mut self, hw_type: HardwareType, sensor_type: SensorType) -> e_utils::AnyResult<Vec<Sensor>> {
    let hw_types = hw_type.all();
    let sensor_types = sensor_type.all();
    let res: Vec<Vec<Sensor>> = hw_types
      .into_iter()
      .flat_map(|hwt| match hwt {
        HardwareType::CPU => Some(self.query_cpu(&sensor_types, &hwt)),
        HardwareType::RAM => Some(self.query_memory(&sensor_types, &hwt)),
        _ => {
          crate::dp(format!("OS HW type {} is not supported", hwt));
          None
        }
      })
      .collect();
    if res.is_empty() {
      Err("OS No sensors found".into())
    } else {
      Ok(res.concat())
    }
  }

  fn query_memory(&mut self, sts: &Vec<SensorType>, parent: &HardwareType) -> Vec<Sensor> {
    sts
      .into_iter()
      .enumerate()
      .flat_map(|(index, st)| match st {
        SensorType::GBData => {
          self.0.refresh_memory_specifics(MemoryRefreshKind::nothing().with_ram());
          let total = bytes_to_gib(self.0.total_memory()).round();
          let used = bytes_to_gib(self.0.used_memory());
          let free = bytes_to_gib(self.0.free_memory());
          Some(vec![Sensor {
            Name: "MemoryTotal".into(),
            Identifier: "MemoryTotal".into(),
            _SensorType: st.to_string(),
            SensorType: st.clone(),
            Value: total,
            data: total.to_string(),
            Min: free,
            Max: used,
            Index: index as i32,
            Parent: parent.to_string(),
          }])
        }
        SensorType::GBSmallData => {
          self.0.refresh_memory_specifics(MemoryRefreshKind::nothing().with_swap());
          let total = bytes_to_gib(self.0.total_swap()).round();
          let used = bytes_to_gib(self.0.used_swap());
          let free = bytes_to_gib(self.0.free_swap());
          Some(vec![Sensor {
            Name: "SwapTotal".into(),
            Identifier: "SwapTotal".into(),
            _SensorType: st.to_string(),
            SensorType: st.clone(),
            Value: total,
            data: total.to_string(),
            Min: free,
            Max: used,
            Index: index as i32,
            Parent: parent.to_string(),
          }])
        }
        SensorType::Load => {
          self.0.refresh_memory_specifics(MemoryRefreshKind::nothing().with_ram());
          let total = self.0.total_memory() as f64;
          let used = self.0.used_memory() as f64;
          let load = (used / total * 100.0).round();
          Some(vec![Sensor {
            Name: "MemoryLoad".into(),
            Identifier: "MemoryLoad".into(),
            _SensorType: st.to_string(),
            SensorType: st.clone(),
            Value: load,
            data: load.to_string(),
            Min: 0.0,
            Max: 100.0,
            Index: index as i32,
            Parent: parent.to_string(),
          }])
        }
        _ => {
          crate::dp(format!("OS Sensor type {} is not supported", st));
          None
        }
      })
      .flatten()
      .collect()
  }
  fn query_cpu(&mut self, sts: &Vec<SensorType>, parent: &HardwareType) -> Vec<Sensor> {
    sts
      .into_iter()
      .flat_map(|st| match st {
        SensorType::Clock => {
          self.0.refresh_cpu_specifics(CpuRefreshKind::nothing().with_frequency());
          let res: Vec<Sensor> = self
            .0
            .cpus()
            .iter()
            .enumerate()
            .map(|(i, cpu)| {
              let value = cpu.frequency() as f64;
              Sensor {
                Name: cpu.name().into(),
                Identifier: cpu.vendor_id().into(),
                _SensorType: st.to_string(),
                SensorType: st.clone(),
                Parent: parent.to_string(),
                Value: value,
                Min: value,
                Max: value,
                Index: i as i32,
                data: value.to_string(),
              }
            })
            .collect();
          Some(res)
        }
        SensorType::Load => {
          self.0.refresh_cpu_specifics(CpuRefreshKind::nothing().with_cpu_usage());
          let res: Vec<Sensor> = self
            .0
            .cpus()
            .iter()
            .enumerate()
            .map(|(i, cpu)| {
              let value = cpu.cpu_usage() as f64;
              Sensor {
                Name: cpu.name().into(),
                Identifier: cpu.vendor_id().into(),
                _SensorType: st.to_string(),
                SensorType: st.clone(),
                Parent: parent.to_string(),
                Value: value,
                Min: value,
                Max: value,
                Index: i as i32,
                data: value.to_string(),
              }
            })
            .collect();
          Some(res)
        }
        _ => {
          crate::dp(format!("OS Sensor type {} is not supported", st));
          None
        }
      })
      .flatten()
      .collect()
  }
}
