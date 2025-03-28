#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
use crate::{
  api_test::{HardwareType, Sensor, SensorType},
  wmic::{Hardware, HardwareMonitor},
};
use e_utils::AnyResult;
use std::{collections::HashMap, str::FromStr as _};
use wmi::{COMLibrary, Variant, WMIConnection};
pub type RawQuery = HashMap<String, Variant>;

/// LHM
#[derive(Clone, Debug)]
pub struct LHM(WMIConnection);
impl LHM {
  pub const EXE: &'static str = "LibreHardwareMonitor.exe";
  pub const DIR: &'static str = "plugins/LHM";
  /// 获取WMI连接
  pub fn get(&self) -> &WMIConnection {
    &self.0
  }
  fn build_query(&self, hw_type: HardwareType) -> String {
    if hw_type == HardwareType::ALL {
      Self::HW_QUERY.to_string()
    } else {
      format!("{} WHERE HardwareType='{}'", Self::HW_QUERY, hw_type)
    }
  }

  fn build_sensor_query(&self, sensor_type: &SensorType) -> String {
    // 构建传感器查询条件
    if sensor_type == &SensorType::ALL {
      String::new()
    } else {
      format!("AND SensorType='{}'", sensor_type)
    }
  }
  /// 异步查询
  pub async fn a_query(&self, hw_type: HardwareType, sensor_type: SensorType) -> AnyResult<Vec<Sensor>> {
    // 查询硬件
    let hws = self
      .get()
      .async_raw_query(self.build_query(hw_type))
      .await?
      .into_iter()
      .map(|mut hw: Hardware<HardwareType>| {
        hw.HardwareType = HardwareType::from_str(&hw._HardwareType).unwrap_or(HardwareType::Unknown);
        hw
      })
      .collect::<Vec<_>>();
    // 构建传感器查询条件
    let st_query = self.build_sensor_query(&sensor_type);
    // 查询传感器
    let mut sensors = Vec::new();
    for hw in hws {
      let query = format!("{} WHERE Parent='{}' {}", Self::SENSOR_QUERY, hw.Identifier, st_query);
      sensors.extend(self.get().async_raw_query(query).await?.into_iter().map(|mut sensor: Sensor| {
        sensor.SensorType = SensorType::from_str(&sensor._SensorType).unwrap_or(SensorType::Unknown);
        sensor
      }));
    }
    if sensors.is_empty() {
      return Err("No sensors found".into());
    }
    Ok(sensors)
  }
  /// 同步查询
  pub fn query(&self, hw_type: HardwareType, sensor_type: SensorType) -> AnyResult<Vec<Sensor>> {
    // 查询硬件
    let hws = self
      .get()
      .raw_query(self.build_query(hw_type))?
      .into_iter()
      .map(|mut hw: Hardware<HardwareType>| {
        hw.HardwareType = HardwareType::from_str(&hw._HardwareType).unwrap_or(HardwareType::Unknown);
        hw
      })
      .collect::<Vec<_>>();
    // 构建传感器查询条件
    let st_query = self.build_sensor_query(&sensor_type);
    // 查询传感器
    let mut sensors = Vec::new();
    for hw in hws {
      let query = format!("{} WHERE Parent='{}' {}", Self::SENSOR_QUERY, hw.Identifier, st_query);
      sensors.extend(self.get().raw_query(query)?.into_iter().map(|mut sensor: Sensor| {
        sensor.SensorType = SensorType::from_str(&sensor._SensorType).unwrap_or(SensorType::Unknown);
        sensor
      }));
    }
    if sensors.is_empty() {
      return Err("No sensors found".into());
    }
    Ok(sensors)
  }
}
impl HardwareMonitor for LHM {
  type HWType = HardwareType;
  type SensorType = Sensor;
  const CON_QUERY: &'static str = "root/LibreHardwareMonitor";
  const HW_QUERY: &'static str = "SELECT * FROM Hardware";
  const SENSOR_QUERY: &'static str = "SELECT * FROM Sensor";
  fn new() -> AnyResult<Self> {
    let com_con = COMLibrary::new()?;
    let wmi = WMIConnection::with_namespace_path(Self::CON_QUERY, com_con)?;
    Ok(LHM(wmi))
  }
  fn test(count: u64) -> AnyResult<()> {
    for i in 1..=count {
      match Self::new() {
        Ok(api) => {
          let has_value = [
            (HardwareType::CPU, SensorType::Clock),
            (HardwareType::ALL, SensorType::Temperature),
            (HardwareType::ALL, SensorType::Fan),
          ]
          .into_iter()
          .any(|(hw_type, sensor_type)| {
            api
              .query(hw_type, sensor_type)
              .ok()
              .and_then(|v| v.first().cloned())
              .map(|v| v.Value != 0.0)
              .unwrap_or(false)
          });

          if has_value {
            crate::dp(format!("Loading... ({}%/{}%)", count, count));
            crate::dp("LibreHardwareMonitor ready");
            return Ok(());
          }
        }
        Err(e) => crate::wp(e.to_string()),
      }
      crate::dp(format!("Loading... ({}%/{}%)", i, count));
      std::thread::sleep(std::time::Duration::from_millis(200));
    }
    Err("LibreHardwareMonitor load timeout".into())
  }
  fn stop() -> AnyResult<()> {
    Ok(())
  }
  fn clean() -> AnyResult<()> {
    Ok(())
  }
}
