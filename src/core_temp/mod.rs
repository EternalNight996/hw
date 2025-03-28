#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
use crate::{
  api_test::{HardwareType, Sensor, SensorType},
  wmic::HardwareMonitor,
};
use csv::StringRecord;
use e_utils::AnyResult;
use std::fs::File;
use std::path::Path;
use std::{fs, io::BufReader};
use std::{
  io::{self},
  path::PathBuf,
};

#[derive(Debug, Default)]
pub struct CoreTemp {
  pub temperatures: Vec<f64>, // 核心温度
  pub frequencys: Vec<f64>,   // CPU主频（MHz）
  pub loads: Vec<f64>,        // CPU负载（%）
  pub powers: Vec<f64>,       // 功耗（W）
}
impl CoreTemp {
  pub fn read_log_path() -> AnyResult<PathBuf> {
    // 读取最新生成的日志文件
    let mut latest_log = None;
    let entries = fs::read_dir(".")?;
    for entry in entries {
      if let Ok(entry) = entry {
        let path = entry.path();
        if path.is_file() {
          if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with("CT-Log") && name.ends_with(".csv") {
              let metadata = fs::metadata(&path)?;
              let modified = metadata.modified()?;
              if latest_log.as_ref().map_or(true, |(t, _)| modified > *t) {
                latest_log = Some((modified, path));
              }
            }
          }
        }
      }
    }
    if let Some((_, path)) = latest_log {
      Ok(path)
    } else {
      Err(io::Error::new(io::ErrorKind::NotFound, "No CT-Log file found").into())
    }
  }
}

impl CoreTemp {
  pub fn parse_log<P: AsRef<Path>>(path: P) -> e_utils::AnyResult<Vec<StringRecord>> {
    // Reset reader to header position
    let file = File::open(path)?;
    let mut rdr = csv::ReaderBuilder::new()
      .flexible(true)
      .has_headers(true)
      .delimiter(b',')
      .from_reader(BufReader::new(file));
    let mut start = false;
    let mut datas = vec![];
    for result in rdr.records() {
      // The iterator yields Result<StringRecord, Error>, so we check the
      // error here.
      match result {
        Ok(record) => {
          if let Some(data) = record.get(0) {
            if data == "Session start:" {
              start = true;
              continue;
            } else if data == "Session end:" {
              start = false;
              continue;
            }
          }
          if start {
            datas.push(record);
          }
        }
        Err(_) => {
          // crate::wp(format!("Error: {}", e))
        }
      }
    }
    Ok(datas)
  }
}

impl CoreTemp {
  pub fn from_csv(row: csv::StringRecord) -> Self {
    let num_cores = (0..)
      .take_while(|&i| row.get(i + 1).and_then(|s| s.parse::<f64>().ok()).map_or(false, |t| t > 0.0 && t < 150.0))
      .count();
    // 温度校验：只保留0-150之间的合理温度值
    let temperatures: Vec<f64> = (0..num_cores)
      .filter_map(|i| row.get(i + 1))
      .filter_map(|s| s.parse::<f64>().ok())
      .filter(|&t| t > 0.0 && t < 150.0)
      .collect();

    // 负载校验：修正列索引为6 + i*5
    let loads: Vec<f64> = (0..num_cores)
      .filter_map(|i| row.get(6 + i * 5))
      .filter_map(|s| s.parse().ok())
      .filter(|&l| l >= 0.0 && l <= 100.0)
      .collect();

    // 频率校验：修正列索引为9 + i*5
    let freqs: Vec<f64> = (0..num_cores)
      .filter_map(|i| row.get(7 + i * 5)) // Columns 9, 14, 19, 24, 29, 34
      .filter_map(|s| s.parse().ok())
      .filter(|&f| f >= 500.0) // Validate reasonable frequency range (MHz)
      .collect();

    // 功耗校验：修正为倒数第二列（假设倒数第一列可能是空）
    let power = vec![row
      .get(row.len().saturating_sub(2))
      .and_then(|s| s.parse().ok())
      .filter(|&p| p >= 0.0 && p <= 500.0)
      .unwrap_or_default()];

    Self {
      temperatures,
      frequencys: freqs,
      loads: loads,
      powers: power,
    }
  }
}

impl CoreTemp {
  pub const EXE: &'static str = "CoreTemp.exe";
  fn parse_value(value: Vec<f64>) -> (f64, f64, f64) {
    let temperature = value.iter().sum::<f64>() / value.len() as f64;
    let min = value.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).cloned().unwrap_or(0.0);
    let max = value.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).cloned().unwrap_or(0.0);
    (temperature, min, max)
  }
  pub fn query(&self, hw_type: HardwareType, stype: SensorType) -> AnyResult<Vec<Sensor>> {
    let path = Self::read_log_path()?;
    let records = Self::parse_log(path)?;
    let mut sensors = vec![];
    let mut i = 0;
    for record in records {
      i += 1;
      let target = match hw_type {
        HardwareType::ALL | HardwareType::CPU => match stype {
          SensorType::Temperature => {
            let core_temp = Self::from_csv(record);
            core_temp.temperatures
          }
          SensorType::Voltage => {
            let core_temp = Self::from_csv(record);
            core_temp.powers
          }
          SensorType::Clock => {
            let core_temp = Self::from_csv(record);
            core_temp.frequencys
          }
          SensorType::Load => {
            let core_temp = Self::from_csv(record);
            core_temp.loads
          }
          SensorType::Power => {
            let core_temp = Self::from_csv(record);
            core_temp.powers
          }
          SensorType::ALL => {
            let core_temp = Self::from_csv(record);
            if core_temp.temperatures.len() > 0 {
              core_temp.temperatures
            } else if core_temp.powers.len() > 0 {
              core_temp.powers
            } else if core_temp.loads.len() > 0 {
              core_temp.loads
            } else if core_temp.frequencys.len() > 0 {
              core_temp.frequencys
            } else {
              vec![]
            }
          }
          _ => return Err(format!("CoreTemp {} {} not supported", hw_type, stype).into()),
        },
        _ => return Err(format!("CoreTemp {} not supported", hw_type).into()),
      };

      let (value, min, max) = Self::parse_value(target);
      let sensor = Sensor {
        Name: format!("CoreTemp"),
        Identifier: format!("CoreTemp"),
        _SensorType: stype.to_string(),
        SensorType: stype.clone(),
        Parent: hw_type.to_string(),
        Value: value,
        Min: min,
        Max: max,
        Index: i,
        data: String::new(),
      };
      sensors.push(sensor);
    }
    Ok(sensors)
  }
}

impl HardwareMonitor for CoreTemp {
  type HWType = HardwareType;
  type SensorType = Sensor;
  const CON_QUERY: &'static str = "";
  const HW_QUERY: &'static str = "";
  const SENSOR_QUERY: &'static str = "";

  fn new() -> AnyResult<Self> {
    Ok(Self::default())
  }

  fn test(count: u64) -> AnyResult<()> {
    for i in 1..=count {
      match Self::new() {
        Ok(api) => {
          let has_value = [(HardwareType::CPU, SensorType::Clock), (HardwareType::ALL, SensorType::Temperature)]
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
            crate::dp("CoreTemp ready");
            return Ok(());
          }
        }
        Err(e) => crate::wp(e.to_string()),
      }
      crate::dp(format!("Loading... ({}%/{}%)", i, count));
      std::thread::sleep(std::time::Duration::from_millis(200));
    }
    Err("CoreTemp load timeout".into())
  }

  fn stop() -> AnyResult<()> {
    Ok(())
  }

  fn clean() -> AnyResult<()> {
    // 清理旧日志文件
    let entries = fs::read_dir(".")?;
    for entry in entries {
      if let Ok(entry) = entry {
        let path = entry.path();
        if path.is_file() {
          if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with("CT-Log") && name.ends_with(".csv") {
              fs::remove_file(path).ok();
            }
          }
        }
      }
    }
    Ok(())
  }
}
