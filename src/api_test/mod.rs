mod inner;
pub use inner::*;
use std::sync::atomic::Ordering;

use serde::Serialize;
use strum::EnumMessage;

#[derive(Serialize)]
pub struct Tester {
  #[serde(skip)]
  pub inner: Inner,
  pub core: TestCore,
}

#[derive(Serialize)]
pub struct TestCore {
  pub results: TestResults,
  pub params: TestParams,
  pub core_count: usize,
  pub is_full: bool,
  pub is_check: bool,
  pub is_print: bool,
  pub is_data: bool,
}
impl Tester {
  #[cfg(feature = "cli")]
  pub fn from_opts(op: &crate::cli::Opts) -> e_utils::AnyResult<Self> {
    use std::str::FromStr as _;
    let hw_type = op.args.get(0).and_then(|v| HardwareType::from_str(v).ok()).unwrap_or_default();
    let sensor_type = op.args.get(1).and_then(|v| SensorType::from_str(v).ok()).unwrap_or_default();
    let test_secs = op.command.get(0).and_then(|v| v.parse().ok()).unwrap_or(1);
    let v1 = op.command.get(1).and_then(|v| v.parse().ok()).unwrap_or(0.0);
    let v2 = op.command.get(2).and_then(|v| v.parse().ok()).unwrap_or(0.0);
    let v3 = op.command.get(3).and_then(|v| v.parse().ok()).unwrap_or(0.0);
    let params = TestParams { test_secs, v1, v2, v3 };
    let is_print = op.task == "print";
    let is_check = op.task == "check";
    let is_data = op.task == "data";
    let mut results = TestResults::new();
    results.api = op.api.to_string();
    results.hw_type = hw_type;
    results.sensor_type = sensor_type;
    Ok(Self {
      inner: Inner::from_api(op.api)?,
      core: TestCore {
        results,
        params,
        core_count: 0,
        is_full: op.full,
        is_check,
        is_print,
        is_data,
      },
    })
  }
}
impl TestCore {
  pub fn set_check(&mut self, is_check: bool) -> &mut Self {
    self.is_check = is_check;
    self
  }
  pub fn hw_str(&self) -> String {
    self.results.hw_type.to_string()
  }
  pub fn hw_name(&self) -> &'static str {
    self.results.hw_type.get_message().unwrap_or_default()
  }
  pub fn sensor_str(&self) -> String {
    self.results.sensor_type.to_string()
  }
  pub fn sensor_name(&self) -> &'static str {
    self.results.sensor_type.get_message().unwrap_or_default()
  }
  pub fn sensor_unit(&self) -> &'static str {
    self.results.sensor_type.unit()
  }
  pub fn get_test_start(&self) -> String {
    format!(
      "\n=== 开始 {} {} {} ===\n\
       --- 传感器 -> {} {} {} ---\n\
       目标: {:.1}\n\
       允许误差: ±{:.1}\n\
       测试时长: {} 秒\n\
       ====================================",
      self.hw_str(),
      self.hw_name(),
      if self.is_check { "测试" } else { "获取" },
      self.sensor_str(),
      self.sensor_name(),
      self.sensor_unit(),
      self.params.v1,
      self.params.v2,
      self.params.test_secs
    )
  }

  pub fn get_test_summary(&self) -> String {
    let stats_info = if self.is_full {
      format!(
        "样本数量: {}\n\
             标准差: {:.2} {}\n",
        self.results.samples,
        self.results.std_deviation(),
        self.sensor_unit()
      )
    } else {
      String::new()
    };

    format!(
      "\n=== 总结 -> {} {} ===\n\
         --- 传感器 -> {} {} {} ---\n\
         结果: {}\n\
         {}\
         数据: {}\n\
         目标: {:.1} {}\n\
         平均: {:.1} {}\n\
         最低: {:.1} {}\n\
         最高: {:.1} {}\n\
         次数: {}\n\
         错误次数: {}\n\
         负载: {:.1}%\n\
         平均负载: {:.1}%\n\
         允许误差: ±{:.1}\n\
         允许范围: {:.1} ~ {:.1} {}\n\
         ====================\n",
      self.hw_str(),
      self.hw_name(),
      self.sensor_str(),
      self.sensor_name(),
      self.sensor_unit(),
      self.results.res,
      stats_info,
      self.results.data,
      self.params.v1,
      self.sensor_unit(),
      self.results.avg,
      self.sensor_unit(),
      self.results.min,
      self.sensor_unit(),
      self.results.max,
      self.sensor_unit(),
      self.params.test_secs,
      self.results.error_count,
      self.params.v3,
      self.results.load.avg,
      self.params.v2,
      self.params.v1 - self.params.v2,
      self.params.v1 + self.params.v2,
      self.sensor_unit(),
    )
  }

  pub fn update_test_status(&mut self, current_sec: usize, sensors: &[Sensor]) -> e_utils::AnyResult<()> {
    crate::dp(format!("\n--- 第 {} 秒{}状态 ---", current_sec + 1, self.hw_name()));

    for (_idx, sensor) in sensors.iter().enumerate() {
      self.results.update(sensor.Value);
      self.results.data = if !sensor.data.is_empty() {
        sensor.data.clone()
      } else {
        self.results.avg.to_string()
      };

      let full = if self.is_full {
        format!("最小={:.1} 最大={:.1} 标识={} ", sensor.Min, sensor.Max, sensor.Identifier,)
      } else {
        String::new()
      };
      let check = if self.is_check {
        format!("误差: ±{:.1}", self.params.v2,)
      } else {
        String::new()
      };
      let s = format!("{} - 当前={:.1} {} {}{}", sensor.Name, sensor.Value, sensor.sensor_unit(), full, check);
      crate::p(s);
      if self.is_check && is_value_out_of_range(sensor.Value, self.params.v1, self.params.v2) {
        self.results.error_count += 1;
        crate::p(format!("警告：{}超出允许范围！", sensor.Name));

        if self.results.error_count > 2 {
          let err = format!(
            "{} 测试失败：\n\
            - 数据: {}\n\
            - 当前{}: {:.1} {}\n\
            - 目标{}: {:.1} {}\n\
            - 连续错误: {} 次\n\
            - 误差: ±{:.1}\n\
            - 允许范围: {:.1} ~ {:.1} {}",
            self.hw_name(),
            sensor.Name,
            self.results.data,
            sensor.Value,
            sensor.sensor_unit(),
            sensor.Name,
            self.params.v1,
            sensor.sensor_unit(),
            self.params.v1 - self.params.v2,
            self.params.v1 + self.params.v2,
            sensor.sensor_unit(),
            self.params.v2,
            self.results.error_count
          );
          return Err(err.into());
        }
      }
    }
    crate::p("--------------------------------");
    crate::p(format!(
      "平均值（{}{}  {:.1}%）   数据:{}\n",
      self.results.avg,
      self.sensor_unit(),
      self.results.load.avg,
      self.results.data
    ));
    Ok(())
  }
}
/// 运行
impl Tester {
  /// 关闭负载
  pub fn close_load(&self) -> e_utils::Result<()> {
    crate::api_test::LOAD_CONTROLLER.running.store(false, Ordering::SeqCst);
    Ok(())
  }
  /// 启动负载
  #[cfg(feature = "system")]
  pub fn spawn_load(&self) -> e_utils::Result<Vec<std::thread::JoinHandle<()>>> {
    // 启动CPU负载
    LoadController::spawn_load(
      self.core.core_count,
      &self.core.results.hw_type,
      &self.core.results.sensor_type,
      self.core.params.v3,
    )
  }
  #[cfg(any(
    all(feature = "ohm", target_os = "windows"),
    all(feature = "aida64", target_os = "windows"),
    feature = "os"
  ))]
  pub async fn run(mut self) -> e_utils::AnyResult<Self> {
    for i in 0..self.core.params.test_secs {
      tokio::time::sleep(std::time::Duration::from_secs(1)).await;
      let res: e_utils::AnyResult<Vec<Sensor>> = match &mut self.inner {
        #[cfg(all(feature = "ohm", target_os = "windows"))]
        Inner::OHM(ohm) => ohm
          .a_query(self.core.results.hw_type.clone(), self.core.results.sensor_type.clone())
          .await
          .map(|v| v.into_iter().filter(|v| v.Name != "Bus Speed").collect()),
        #[cfg(all(feature = "aida64", target_os = "windows"))]
        Inner::AIDA64(aida64) => {
          aida64
            .a_query(self.core.results.hw_type.clone(), self.core.results.sensor_type.clone())
            .await
        }
        #[cfg(feature = "os")]
        Inner::OS(os) => os.query(self.core.results.hw_type.clone(), self.core.results.sensor_type.clone()),
        _ => return Err("不支持".into()),
      };
      match res {
        Ok(sensors) => self.core.update_test_status(i, &sensors)?,
        Err(e) => {
          if self.core.is_check {
            return Err(e);
          } else {
            crate::p(format!("{}", e));
          }
        }
      }
    }

    Ok(self)
  }
  pub fn get_test_start(&self) -> String {
    self.core.get_test_start()
  }
  pub fn get_test_summary(&self) -> String {
    self.core.get_test_summary()
  }
}

#[inline]
fn is_value_out_of_range(value: f64, target: f64, range: f64) -> bool {
  value > target + range || value < target - range
}
