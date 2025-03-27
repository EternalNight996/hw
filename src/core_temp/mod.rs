#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
use crate::{
  api_test::{HardwareType, Sensor, SensorType},
  wmic::HardwareMonitor,
};
use e_utils::{
  chrono::NaiveDateTime,
  cmd::{Cmd, ExeType},
  AnyResult,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::File};
use std::{
  fs,
  io::{BufRead as _, BufReader, Seek as _},
};
use std::{
  io::{self, Write},
  path::PathBuf,
};
use std::{path::Path, time::Duration};

#[derive(Debug, Default)]
pub struct CoreTemp {
  pub temperatures: Option<f32>, // 核心温度
  pub frequency: Option<f32>,    // CPU主频（MHz）
  pub load: Option<f32>,         // CPU负载（%）
  pub power: Option<f32>,        // 功耗（W）
}
impl CoreTemp {
  pub fn clean_log() -> AnyResult<()> {
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
  pub fn parse_log<P: AsRef<Path>>(path: P) -> e_utils::AnyResult<Vec<Self>> {
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
            datas.push(CoreTemp::from(record));
          }
        }
        Err(e) => {
          println!("Error: {}", e);
        }
      }
    }
    Ok(datas)
  }
}

impl From<csv::StringRecord> for CoreTemp {
  fn from(row: csv::StringRecord) -> Self {
    let num_cores = (0..)
      .take_while(|&i| row.get(i + 1).and_then(|s| s.parse::<f32>().ok()).map_or(false, |t| t > 0.0 && t < 150.0))
      .count();
    // 温度校验：只保留0-150之间的合理温度值
    let temperatures: Vec<f32> = (0..num_cores)
      .filter_map(|i| row.get(i + 1))
      .filter_map(|s| s.parse::<f32>().ok())
      .filter(|&t| t > 0.0 && t < 150.0)
      .collect();
    let temperature_avg = (!temperatures.is_empty()).then(|| temperatures.iter().sum::<f32>() / temperatures.len() as f32);

    // 负载校验：修正列索引为6 + i*5
    let loads: Vec<f32> = (0..num_cores)
      .filter_map(|i| row.get(6 + i * 5))
      .filter_map(|s| s.parse().ok())
      .filter(|&l| l >= 0.0 && l <= 100.0)
      .collect();
    let load_avg = (!loads.is_empty()).then(|| loads.iter().sum::<f32>() / loads.len() as f32);

    // 频率校验：修正列索引为9 + i*5
    let freqs: Vec<f32> = (0..num_cores)
      .filter_map(|i| row.get(7 + i * 5)) // Columns 9, 14, 19, 24, 29, 34
      .filter_map(|s| s.parse().ok())
      .filter(|&f| f >= 500.0) // Validate reasonable frequency range (MHz)
      .collect();
    let freq_avg = (!freqs.is_empty()).then(|| freqs.iter().sum::<f32>() / freqs.len() as f32);

    // 功耗校验：修正为倒数第二列（假设倒数第一列可能是空）
    let power = row
      .get(row.len().saturating_sub(2))
      .and_then(|s| s.parse().ok())
      .filter(|&p| p >= 0.0 && p <= 500.0);

    Self {
      temperatures: temperature_avg,
      frequency: freq_avg,
      load: load_avg,
      power,
    }
  }
}

impl CoreTemp {
  pub const EXE: &'static str = "OpenHardwareMonitor.exe";
}

impl HardwareMonitor for CoreTemp {
  type HWType = HardwareType;
  type SensorType = Sensor;
}
