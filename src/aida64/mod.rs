#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
use crate::{
  api_test::{HardwareType, Sensor, SensorType},
  wmic::HardwareMonitor,
};
use e_utils::{
  cmd::{Cmd, ExeType},
  AnyResult,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wmi::{COMLibrary, Variant, WMIConnection};

/// 原始查询结果
pub type RawQuery = HashMap<String, Variant>;

#[derive(Clone, Debug)]
/// AIDA64
pub struct AIDA64(WMIConnection);
impl AIDA64 {
  pub const EXE: &'static str = "AIDA64.exe";
  pub const DIR: &'static str = "plugins/AIDA64";
  /// 获取WMI连接
  pub fn get(&self) -> &WMIConnection {
    &self.0
  }
  /// 异步查询
  pub async fn a_query(&self, hw_type: HardwareType, stype: SensorType) -> AnyResult<Vec<Sensor>> {
    let query = format!("{}{}", Self::SENSOR_QUERY, Self::build_query_conditions(&hw_type, &stype));
    let src_sensors: Vec<AIDA64Sensor> = self.get().async_raw_query(&query).await?;
    crate::dp(format!("Found {} src sensors", src_sensors.len()));
    let sensors = convert_sensors(src_sensors);
    if sensors.is_empty() {
      return Err(format!("No sensors found for {} {}", hw_type.to_string(), stype.to_string()).into());
    }

    Ok(sensors)
  }
  /// 同步查询
  pub fn query(&self, hw_type: HardwareType, stype: SensorType) -> AnyResult<Vec<Sensor>> {
    let query = format!("{}{}", Self::SENSOR_QUERY, Self::build_query_conditions(&hw_type, &stype));
    let src_sensors: Vec<AIDA64Sensor> = self.get().raw_query(&query)?;
    crate::dp(format!("Found {} src sensors", src_sensors.len()));
    let sensors = convert_sensors(src_sensors);
    if sensors.is_empty() {
      return Err(format!("No sensors found for {} {}", hw_type.to_string(), stype.to_string()).into());
    }

    Ok(sensors)
  }
}
impl AIDA64 {
  /// 解析硬件类型为查询条件
  fn parse_hw_type(hw_type: &HardwareType) -> Vec<String> {
    match hw_type {
      HardwareType::CPU => vec!["CPU"],
      HardwareType::GpuNvidia => vec!["NVIDIA"],
      HardwareType::GpuAti => vec!["ATI", "AMD"],
      HardwareType::RAM => vec!["Memory", "DIMM"],
      HardwareType::Mainboard => vec!["Motherboard", "MOBO"],
      HardwareType::HDD => vec!["Disk", "Drive"],
      _ => vec![], // 其他类型不添加特定条件
    }
    .into_iter()
    .map(|s| format!("Label LIKE '%{}%'", s))
    .collect()
  }

  /// 解析传感器类型为查询条件
  fn parse_sensor_type(stype: &SensorType) -> (Option<String>, Vec<String>) {
    match stype {
      // 直接类型匹配
      SensorType::Temperature => (Some("T".into()), vec![]),
      SensorType::Voltage => (Some("V".into()), vec![]),
      SensorType::Power => (Some("P".into()), vec![]),
      SensorType::Fan => (Some("F".into()), vec![]),

      // S类型需要额外的Label条件
      SensorType::Clock => (
        Some("S".into()),
        vec!["Label LIKE '%Core #%'", "Label LIKE '%Clock%'"].into_iter().map(String::from).collect(),
      ),
      SensorType::ClockAverage => (Some("S".into()), vec!["Label = 'CPU Clock'"].into_iter().map(String::from).collect()),
      SensorType::Load => (
        Some("S".into()),
        vec!["Label LIKE '%Utilization%' OR Label LIKE '%Activity%'"]
          .into_iter()
          .map(String::from)
          .collect(),
      ),
      SensorType::FSB => (Some("S".into()), vec!["Label LIKE '%FSB%'"].into_iter().map(String::from).collect()),
      SensorType::Multiplexer => (Some("S".into()), vec!["Label LIKE '%Multiplier%'"].into_iter().map(String::from).collect()),
      SensorType::DataRate => (
        Some("S".into()),
        vec!["Label LIKE '%Rate%' OR Label LIKE '%Speed%'"].into_iter().map(String::from).collect(),
      ),
      SensorType::Data | SensorType::GBData => (Some("S".into()), vec!["Label LIKE '%Memory%'"].into_iter().map(String::from).collect()),
      SensorType::Flow => (Some("S".into()), vec!["Label LIKE '%Flow%'"].into_iter().map(String::from).collect()),
      SensorType::Control => (Some("S".into()), vec!["Label LIKE '%Control%'"].into_iter().map(String::from).collect()),
      SensorType::Level => (Some("S".into()), vec!["Label LIKE '%Level%'"].into_iter().map(String::from).collect()),
      SensorType::Throughput => (Some("S".into()), vec!["Label LIKE '%Throughput%'"].into_iter().map(String::from).collect()),
      SensorType::SmallData | SensorType::GBSmallData => (Some("S".into()), vec!["Label LIKE '%Small%'"].into_iter().map(String::from).collect()),
      SensorType::ALL => (None, vec![]),
      SensorType::Unknown => (None, vec!["Type NOT IN ('S', 'T', 'F', 'V', 'P')"].into_iter().map(String::from).collect()),
    }
  }

  /// 构建查询条件
  fn build_query_conditions(hw_type: &HardwareType, stype: &SensorType) -> String {
    let mut conditions = Vec::new();

    // 添加硬件类型条件
    let hw_conditions = Self::parse_hw_type(hw_type);
    if !hw_conditions.is_empty() {
      conditions.push(format!("({})", hw_conditions.join(" OR ")));
    }

    // 添加传感器类型条件
    let (type_pattern, extra_conditions) = Self::parse_sensor_type(stype);
    if let Some(type_str) = type_pattern {
      conditions.push(format!("Type = '{}'", type_str));
    }
    conditions.extend(extra_conditions);

    // 构建最终的查询条件
    if conditions.is_empty() {
      String::new()
    } else {
      format!(" WHERE {}", conditions.join(" AND "))
    }
  }
}
impl HardwareMonitor for AIDA64 {
  type HWType = HardwareType;
  type SensorType = Sensor;
  const CON_QUERY: &'static str = "root\\WMI";
  const HW_QUERY: &'static str = "SELECT * FROM Hardware";
  const SENSOR_QUERY: &'static str = "SELECT * FROM AIDA64_SensorValues";
  fn new() -> AnyResult<Self> {
    let com_con = COMLibrary::new()?;
    let wmi = WMIConnection::with_namespace_path(Self::CON_QUERY, com_con)?;
    Ok(AIDA64(wmi))
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
            crate::dp("AIDA64 ready");
            return Ok(());
          }
        }
        Err(e) => crate::wp(e.to_string()),
      }
      crate::dp(format!("Loading... ({}%/{}%)", i, count));
      std::thread::sleep(std::time::Duration::from_millis(200));
    }
    Err("AIDA64 load timeout".into())
  }
  fn stop() -> AnyResult<()> {
    if cfg!(target_os = "windows") {
      let res = Cmd::new("sc")
        .set_type(ExeType::Cmd)
        .args(&["config", "AIDA64Driver", "start=", "disabled"])
        .output()?;
      crate::dp(format!("AIDA64 [AIDA64Driver kerneld.*] disable: {}", res.stdout));
      let res = Cmd::new("sc").set_type(ExeType::Cmd).args(&["stop", "AIDA64Driver"]).output()?;
      crate::dp(format!("AIDA64 [AIDA64Driver kerneld.*] stop: {}", res.stdout));
    }
    Ok(())
  }
  fn clean() -> AnyResult<()> {
    if cfg!(target_os = "windows") {
      let res = Cmd::new("sc").set_type(ExeType::Cmd).args(&["delete", "AIDA64Driver"]).output()?;
      crate::dp(format!("AIDA64 [AIDA64Driver kerneld.*]  delete: {}", res.stdout));
    }
    Ok(())
  }
}

/// 表示 OpenHardwareMonitor WMI 提供程序中的传感器
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AIDA64Sensor {
  /// 传感器ID
  pub ID: String,
  /// 传感器值
  pub Value: String,
  /// 传感器标签
  pub Label: String,
  /// 传感器类型
  pub Type: String,
}
impl AIDA64Sensor {
  /// 解析传感器类型
  pub fn parse_sensor_type(&self) -> SensorType {
    match self.Type.as_bytes().first().copied().unwrap_or(b'X') {
      b'T' => SensorType::Temperature,
      b'V' => SensorType::Voltage,
      b'P' => SensorType::Power,
      b'F' => SensorType::Fan,
      b'C' => SensorType::Control,
      b'D' => SensorType::Data,
      b'S' => {
        // CPU相关传感器的快速匹配
        if self.Label.contains("CPU") {
          if self.Label.contains("Core #") && self.Label.contains("Clock") {
            SensorType::Clock // CPU Core #N Clock
          } else if self.Label == "CPU Clock" {
            SensorType::ClockAverage // CPU Clock (整体频率)
          } else if self.Label.contains("FSB") {
            SensorType::FSB // CPU FSB
          } else if self.Label.contains("Multiplier") {
            SensorType::Multiplexer // CPU Multiplier
          } else if self.Label.contains("Utilization") {
            SensorType::Load // CPU Utilization
          } else {
            SensorType::Data
          }
        }
        // 其他传感器类型的快速匹配
        else if self.Label.contains("Rate") || self.Label.contains("Speed") {
          SensorType::DataRate // Download Rate, Upload Rate, Read Speed, Write Speed
        } else if self.Label.contains("Space") || self.Label.contains("Size") {
          SensorType::Data // Used Space, Free Space, Total Size
        } else if self.Label.contains("Memory") {
          SensorType::Data // Used Memory, Free Memory
        } else if self.Label.contains("Activity") {
          SensorType::Load // Disk Activity
        } else if self.Label.contains("Level") {
          SensorType::Level // Battery Level
        } else {
          SensorType::Data
        }
      }
      _ => SensorType::Data,
    }
  }

  /// 解析硬件类型
  pub fn parse_hardware_type(&self) -> HardwareType {
    if self.Label.contains("CPU") {
      HardwareType::CPU
    } else if self.Label.contains("GPU") || self.Label.contains("Graphics") {
      if self.Label.contains("NVIDIA") {
        HardwareType::GpuNvidia
      } else if self.Label.contains("ATI") || self.Label.contains("AMD") {
        HardwareType::GpuAti
      } else {
        HardwareType::ALL
      }
    } else if self.Label.contains("Memory") || self.Label.contains("DIMM") {
      HardwareType::RAM
    } else if self.Label.contains("Motherboard") || self.Label.contains("MOBO") {
      HardwareType::Mainboard
    } else if self.Label.contains("Disk") || self.Label.contains("Drive") {
      HardwareType::HDD
    } else {
      HardwareType::ALL
    }
  }
  /// 转换为Sensor
  pub fn to_sensor(&self) -> Sensor {
    let stype = self.parse_sensor_type();
    let hardware_type = self.parse_hardware_type();

    let value = if self.Value == "TRIAL" { 0.0 } else { self.Value.parse().unwrap_or_default() };

    Sensor {
      Name: self.Label.clone(),
      Identifier: self.ID.clone(),
      _SensorType: stype.to_string(),
      SensorType: stype,
      Parent: hardware_type.to_string(),
      Value: value,
      Min: 0.0,
      Max: 0.0,
      Index: 0,
      data: String::new(),
    }
  }
}
#[inline]
/// 批量转换
pub fn convert_sensors(aida_sensors: Vec<AIDA64Sensor>) -> Vec<Sensor> {
  aida_sensors.into_iter().map(|s| s.to_sensor()).collect()
}
