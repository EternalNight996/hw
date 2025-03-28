#![allow(unused)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
use std::sync::{
  atomic::{AtomicBool, AtomicU64, Ordering},
  Arc,
};

use e_utils::once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use strum::*;

/// 全局负载控制器
pub static LOAD_CONTROLLER: Lazy<LoadController> = Lazy::new(|| LoadController::new(50));
/// 扩展1
pub const EXTEND1: i32 = 0x1;

#[doc(hidden)]
#[derive(Debug)]
pub enum Inner {
  #[cfg(all(feature = "ohm", target_os = "windows"))]
  OHM(crate::ohm::OHM),
  #[cfg(all(feature = "aida64", target_os = "windows"))]
  AIDA64(crate::aida64::AIDA64),
  #[cfg(feature = "os")]
  OS(crate::os::OS),
  OSMore,
  Drive,
  FileInfo,
  OSSystem,
  OSOffice,
  Disk,
  #[cfg(all(feature = "core-temp", target_os = "windows"))]
  CoreTemp(crate::core_temp::CoreTemp),
}
impl Inner {
  /// 从API创建Inner
  #[cfg(feature = "cli")]
  pub fn from_api(api: crate::OptsApi) -> e_utils::AnyResult<Self> {
    use crate::{wmic::HardwareMonitor as _, OptsApi};
    match api {
      #[cfg(all(feature = "ohm", target_os = "windows"))]
      OptsApi::OHM => Ok(Self::OHM(crate::ohm::OHM::new()?)),
      #[cfg(not(all(feature = "ohm", target_os = "windows")))]
      OptsApi::OHM => Err("OHM not supported".into()),
      #[cfg(all(feature = "aida64", target_os = "windows"))]
      OptsApi::AIDA64 => Ok(Self::AIDA64(crate::aida64::AIDA64::new()?)),
      #[cfg(not(all(feature = "aida64", target_os = "windows")))]
      OptsApi::AIDA64 => Err("AIDA64 not supported".into()),
      #[cfg(feature = "os")]
      OptsApi::OS => Ok(Self::OS(crate::os::OS::new())),
      #[cfg(not(feature = "os"))]
      OptsApi::OS => Err("OS not supported".into()),
      OptsApi::OSMore => Ok(Self::OSMore),
      OptsApi::Drive => Ok(Self::Drive),
      OptsApi::FileInfo => Ok(Self::FileInfo),
      OptsApi::OSSystem => Ok(Self::OSSystem),
      OptsApi::OSOffice => Ok(Self::OSOffice),
      OptsApi::Disk => Ok(Self::Disk),
      #[cfg(all(feature = "core-temp", target_os = "windows"))]
      OptsApi::CoreTemp => Ok(Self::CoreTemp(crate::core_temp::CoreTemp::new()?)),
      #[cfg(not(all(feature = "core-temp", target_os = "windows")))]
      OptsApi::CoreTemp => Err("CoreTemp not supported".into()),
    }
  }
}

/// API
impl Inner {
  #[cfg(feature = "system")]
  pub async fn get_cpu_core_count(&self) -> e_utils::AnyResult<usize> {
    let mut sys = sysinfo::System::new();
    sys.refresh_cpu_specifics(sysinfo::CpuRefreshKind::nothing().with_frequency());
    Ok(sys.cpus().len())
  }
  /// 获取全局使用频率
  #[cfg(feature = "system")]
  pub async fn get_global_cpu_usage(&self) -> f32 {
    let mut sys = sysinfo::System::new();
    sys.refresh_cpu_specifics(sysinfo::CpuRefreshKind::nothing().with_cpu_usage());
    sys.global_cpu_usage()
  }
}

#[derive(Debug, Clone)]
pub struct LoadController {
  // 目标负载 (0-100)
  pub loaded: Arc<AtomicU64>,
  // 当前负载
  pub current_load: Arc<AtomicU64>,
  // 当前迭代次数
  pub current_iterations: Arc<AtomicU64>,
  // 是否运行
  pub running: Arc<AtomicBool>,
  /// 总迭代次数
  pub total_iterations: Arc<AtomicU64>,
}

impl LoadController {
  pub fn new(load: u64) -> Self {
    let load = load.clamp(0, 100);
    Self {
      loaded: Arc::new(AtomicU64::new(load)),
      current_load: Arc::new(AtomicU64::new(0)),
      current_iterations: Arc::new(AtomicU64::new(100_000)),
      running: Arc::new(AtomicBool::new(false)),
      total_iterations: Arc::new(AtomicU64::new(0)),
    }
  }
  /// 设置目标负载
  pub fn set_loaded(&self, load: f64) {
    let load = load.clamp(0.0, 100.0) as u64;
    self.loaded.store(load, Ordering::Release);
  }
  // 获取当前迭代次数
  pub fn get_iterations(&self) -> u64 {
    self.current_iterations.load(Ordering::Acquire)
  }

  // 更新当前负载并自动调整迭代次数
  pub fn auto_fix_load(&self, load: f64) {
    let current = load.clamp(0.0, 100.0) as u64;
    self.current_load.store(current, Ordering::Release);
    if load == 100.0 {
      return;
    }
    // 获取目标负载和当前负载
    let target = self.loaded.load(Ordering::Acquire);
    // 只有当负载差异超过5%时才调整
    if (target as i64 - current as i64).abs() > 5 {
      let current_iters = self.current_iterations.load(Ordering::Acquire);
      let new_iters = if current < target {
        // 当前负载小于目标负载，增加迭代次数
        (current_iters as f64 * 1.1) as u64
      } else {
        // 当前负载大于目标负载，减少迭代次数
        (current_iters as f64 * 0.9) as u64
      };

      // 确保迭代次数在合理范围内
      let new_iters = new_iters.clamp(u64::MIN, u64::MAX);
      self.current_iterations.store(new_iters, Ordering::Release);
    }
  }
}

impl LoadController {
  /// 开启
  pub fn start_running(&self) {
    crate::dp("启动负载");
    self.running.store(true, Ordering::SeqCst);
  }
  /// 关闭负载
  pub fn stop_running(&self) {
    crate::dp("关闭负载");
    self.running.store(false, Ordering::SeqCst);
  }
  /// 启动负载
  #[cfg(feature = "system")]
  pub fn spawn_load(core_count: usize, hw_type: &HardwareType, s_type: &SensorType, loaded: f64) -> e_utils::Result<Vec<std::thread::JoinHandle<()>>> {
    LOAD_CONTROLLER.set_loaded(loaded);
    if matches!(
      (hw_type, s_type),
      (HardwareType::CPU, SensorType::Load | SensorType::Clock | SensorType::ClockAverage)
    ) {
      // 设置全局负载
      if loaded == 0.0 {
        return Err(
          format!(
            "Skipping load generation for hw_type: {:?}, sensor_type: {:?}, Load cannot be 0",
            hw_type, s_type
          )
          .into(),
        );
      }
      crate::dp(&format!("硬件:{} 传感器:{} 内核:{} 标准负载:{}% 启动负载", hw_type, s_type, core_count, loaded));
      LOAD_CONTROLLER.start_running();
      Ok(spawn_cpu_load(core_count))
    } else {
      Ok(vec![])
    }
  }
}

/// 生成CPU负载的线程
#[cfg(feature = "system")]
fn spawn_cpu_load(core_count: usize) -> Vec<std::thread::JoinHandle<()>> {
  const ADJUST_INTERVAL: Duration = Duration::from_millis(100);
  const MIN_SLEEP_DURATION: Duration = Duration::from_millis(100);

  (0..core_count)
    .map(|core_id| {
      let running = LOAD_CONTROLLER.running.clone();
      let total_iterations = LOAD_CONTROLLER.total_iterations.clone();

      // 使用 tracing 或 log 替代 println
      crate::dp(format!("核心 {} - 初始化", core_id));

      std::thread::Builder::new()
        .name(format!("cpu-load-{}", core_id)) // 命名线程
        .spawn(move || {
          let mut sys = sysinfo::System::new();
          let mut last_adjust = Instant::now();

          while running.load(Ordering::Relaxed) {
            // 使用 Relaxed 提高性能
            let start_time = Instant::now();

            // 获取并限制迭代次数
            let iterations = LOAD_CONTROLLER.get_iterations();

            // 执行CPU密集计算
            perform_cpu_work(iterations, &total_iterations);

            // 定期调整负载
            if start_time.duration_since(last_adjust) >= ADJUST_INTERVAL {
              adjust_load(&mut sys);
              last_adjust = Instant::now(); // 使用当前时间而不是开始时间
            }

            // 动态调整休眠时间
            if let Some(sleep_time) = MIN_SLEEP_DURATION.checked_sub(start_time.elapsed()) {
              std::thread::sleep(sleep_time);
            }
          }

          crate::dp(format!("核心 {} - 线程结束", core_id));
        })
        .expect("线程创建失败")
    })
    .collect()
}

/// 调整负载
#[cfg(feature = "system")]
fn adjust_load(sys: &mut sysinfo::System) {
  sys.refresh_cpu_specifics(sysinfo::CpuRefreshKind::nothing().with_cpu_usage());
  let cpu_load = (sys.global_cpu_usage() as f64).round().clamp(0.0, 100.0); // 限制范围
  LOAD_CONTROLLER.auto_fix_load(cpu_load);
}

/// CPU密集计算
#[inline(never)]
fn perform_cpu_work(iterations: u64, total_iterations: &AtomicU64) {
  const MAX_X: f64 = f64::MAX; // 添加最大值限制
  let mut x = std::hint::black_box(0.0_f64);
  let mut overflow_check = 0;

  for i in 0..iterations {
    // 添加溢出检查
    x = std::hint::black_box({
      let new_x = (x.sin() + 1.0).sqrt() + (i as f64).cos() * std::f64::consts::PI;
      if new_x.is_finite() && new_x.abs() < MAX_X {
        new_x
      } else {
        overflow_check += 1;
        0.0
      }
    });

    // 检查是否有太多溢出
    if overflow_check > iterations / 2 {
      crate::wp("CPU负载检测到过多的数值溢出");
      break;
    }

    // 使用 Release 序提高性能
    std::sync::atomic::fence(Ordering::Release);
  }

  // 使用 Relaxed 序提高性能
  total_iterations.fetch_add(iterations, Ordering::Relaxed);
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct TestLoadResult {
  pub min: f64,
  pub max: f64,
  pub avg: f64,
  pub total: f64,
  pub status: Vec<(String, f64)>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct TestResults {
  pub api: String,
  pub hw_type: HardwareType,
  pub sensor_type: SensorType,
  pub res: String,
  pub data: String,
  pub min: f64,
  pub max: f64,
  pub avg: f64,
  pub total: f64,
  pub samples: usize,
  pub test_secs: usize,
  pub error_count: usize,
  pub load: TestLoadResult,
  pub status: Vec<(String, f64)>,
}
impl TestResults {
  pub fn new() -> Self {
    TestResults {
      api: String::new(),
      hw_type: HardwareType::default(),
      sensor_type: SensorType::default(),
      res: String::new(),
      data: String::new(),
      min: 0.0,
      max: 0.0,
      avg: 0.0,
      total: 0.0,
      samples: 0,
      test_secs: 0,
      error_count: 0,
      load: TestLoadResult::default(),
      status: Vec::new(),
    }
  }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct TestParams {
  pub test_secs: usize,
  pub v1: f64,
  pub v2: f64,
  pub v3: f64,
}

impl TestResults {
  pub fn update(&mut self, value: f64) {
    self.min = if self.min == 0.0 { value } else { self.min.min(value) };
    self.max = self.max.max(value);
    self.total += value;
    self.samples += 1;
    self.status.push((String::new(), value));
    self.avg = (self.total / self.samples as f64).round();
    let load = LOAD_CONTROLLER.current_load.load(Ordering::SeqCst) as f64;
    self.load.total += load;
    self.load.avg = load;
  }

  pub fn std_deviation(&self) -> f64 {
    if self.samples < 2 {
      return 0.0;
    }
    let mean = self.avg;
    let variance = self.status.iter().map(|x| (x.1 - mean).powi(2)).sum::<f64>() / (self.samples - 1) as f64;
    variance.sqrt()
  }
}

/// 表示不同类型的硬件组件
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Display, EnumString, EnumMessage, Default, VariantArray)]
pub enum HardwareType {
  #[default]
  #[strum(message = "所有硬件")]
  ALL,
  #[strum(message = "主板")]
  Mainboard,
  #[strum(message = "Super I/O 芯片")]
  SuperIO,
  #[strum(message = "中央处理器")]
  CPU,
  #[strum(message = "NVIDIA 图形处理器")]
  GpuNvidia,
  #[strum(message = "AMD/ATI 图形处理器")]
  GpuAti,
  #[strum(message = "T-Balancer 设备")]
  TBalancer,
  #[strum(message = "Heatmaster 设备")]
  Heatmaster,
  #[strum(message = "硬盘驱动器")]
  HDD,
  #[strum(message = "内存")]
  RAM,
  #[strum(message = "未知")]
  Unknown,
}
impl HardwareType {
  pub fn all(self) -> Vec<HardwareType> {
    if self == HardwareType::ALL {
      HardwareType::VARIANTS.iter().filter(|v| *v != &HardwareType::ALL).cloned().collect()
    } else {
      vec![self]
    }
  }
}
/// 表示不同类型的传感器及其关联的单位和显示格式
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Display, EnumString, EnumMessage, Default, VariantArray)]
pub enum SensorType {
  /// 所有传感器
  #[default]
  #[strum(message = "所有传感器")]
  ALL,
  /// 电压传感器 (单位: 伏特, 显示: "{value} V")
  #[strum(message = "电压")]
  Voltage,
  /// 时钟速度传感器 (单位: 兆赫兹, 显示: "{value} MHz")
  #[strum(message = "频率")]
  Clock,
  /// 温度传感器 (单位: 摄氏度, 显示: "{value} °C")
  #[strum(message = "温度")]
  Temperature,
  /// 负载传感器 (单位: 百分比, 显示: "{value}%")
  #[strum(message = "负载")]
  Load,
  /// 风扇速度传感器 (单位: 转/分钟, 显示: "{value} RPM")
  #[strum(message = "风扇")]
  Fan,
  /// 流量传感器 (单位: 升/小时, 显示: "{value} L/h")
  #[strum(message = "流量")]
  Flow,
  /// 控制传感器 (单位: 百分比, 显示: "{value}%")
  #[strum(message = "控制")]
  Control,
  /// 水平传感器 (单位: 百分比, 显示: "{value}%")
  #[strum(message = "等级")]
  Level,
  /// 功率传感器 (单位: 瓦特, 显示: "{value} W")
  #[strum(message = "功率")]
  Power,
  /// 数据传感器 (单位: 字节, 显示: "{value} B")
  #[strum(message = "数据")]
  Data,
  /// 数据传感器 (单位: 字节, 显示: "{value} GB")
  #[strum(message = "GB数据")]
  GBData,
  /// 吞吐量传感器 (单位: 字节/秒, 显示: "{value} B/s")
  #[strum(message = "吞吐量")]
  Throughput,
  /// 数据速率传感器 (单位: 字节/秒, 显示: "{value} B/s")
  #[strum(message = "数据速率")]
  DataRate,
  /// 小数据包传感器 (单位: 字节, 显示: "{value} B")
  #[strum(message = "小数据包")]
  SmallData,
  /// 小数据包传感器 (单位: 字节, 显示: "{value} GB")
  #[strum(message = "GB小数据包")]
  GBSmallData,
  /// 前端总线传感器 (单位: 兆赫兹, 显示: "{value} MHz")
  #[strum(message = "前端总线")]
  FSB,
  /// 多路复用器传感器 (单位: 兆赫兹, 显示: "{value} MHz")
  #[strum(message = "多路复用器")]
  Multiplexer,
  /// 平均时钟速度传感器 (单位: 兆赫兹, 显示: "{value} MHz")
  #[strum(message = "平均频率")]
  ClockAverage,
  /// 未知
  #[strum(message = "未知")]
  Unknown,
}

impl SensorType {
  /// 获取此传感器类型的更多传感器类型
  pub fn all(self) -> Vec<SensorType> {
    if self == SensorType::ALL {
      SensorType::VARIANTS.iter().filter(|v| *v != &SensorType::ALL).cloned().collect()
    } else {
      vec![self]
    }
  }
  /// 获取此传感器类型的测量单位
  pub fn unit(&self) -> &'static str {
    match self {
      SensorType::Voltage => "V",
      SensorType::ClockAverage | SensorType::Clock | SensorType::FSB | SensorType::Multiplexer => "MHz",
      SensorType::Temperature => "°C",
      SensorType::Load => "%",
      SensorType::Fan => "RPM",
      SensorType::Flow => "L/h",
      SensorType::Control => "%",
      SensorType::Level => "%",
      SensorType::Power => "W",
      SensorType::Data => "B",
      SensorType::GBData | SensorType::GBSmallData => "GB",
      SensorType::Throughput => "B/s",
      SensorType::DataRate => "B/s",
      SensorType::SmallData => "SB",
      SensorType::ALL => "*",
      SensorType::Unknown => "*",
    }
  }
}
/// 表示 OpenHardwareMonitor WMI 提供程序中的传感器
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Sensor {
  pub Name: String,
  pub Identifier: String,
  #[serde(rename = "SensorType")]
  pub _SensorType: String,
  #[serde(skip)]
  pub SensorType: SensorType,
  pub Parent: String,
  pub Value: f64,
  pub Min: f64,
  pub Max: f64,
  pub Index: i32,
  #[serde(skip)]
  pub data: String,
}
impl Sensor {
  pub fn sensor_unit(&self) -> String {
    format!("{}({})", self.SensorType.unit(), self.SensorType.get_message().unwrap_or_default())
  }
}
