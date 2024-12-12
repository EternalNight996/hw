/// API
pub mod api;
pub use api::*;
use e_utils::AnyResult;
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use structopt::{clap::arg_enum, StructOpt};

arg_enum! {
    /// Api接口
    #[derive(Deserialize, Serialize, Debug, StructOpt, Clone, PartialEq, Copy)]
    pub enum OptsApi {
        OHM,
        AIDA64,
        OS,
        OSMore,
        Drive,
        FileInfo,
        OSSystem,
        OSOffice,
        Disk,
    }
}

/// e-app
/// ------------------------------------------------------
///
#[derive(StructOpt, Debug, Clone, Serialize, Deserialize)]
#[structopt(name = "", setting = structopt::clap::AppSettings::TrailingVarArg,)]
#[structopt(after_help = r#"

```rust
pub enum HardwareType {
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
pub enum SensorType {
  /// 所有传感器
  ALL,
  /// 电压传感器 (单位: 伏特, 显示: "{value} V")
  Voltage,
  /// 时钟速度传感器 (单位: 兆赫兹, 显示: "{value} MHz")
  Clock,
  /// 温度传感器 (单位: 摄氏度, 显示: "{value} °C")
  Temperature,
  /// 负载传感器 (单位: 百分比, 显示: "{value}%")
  Load,
  /// 风扇速度传感器 (单位: 转/分钟, 显示: "{value} RPM")
  Fan,
  /// 流量传感器 (单位: 升/小时, 显示: "{value} L/h")
  Flow,
  /// 控制传感器 (单位: 百分比, 显示: "{value}%")
  Control,
  /// 水平传感器 (单位: 百分比, 显示: "{value}%")
  Level,
  /// 功率传感器 (单位: 瓦特, 显示: "{value} W")
  Power,
  /// 数据传感器 (单位: 字节, 显示: "{value} B")
  Data,
  /// 数据传感器 (单位: 字节, 显示: "{value} GB")
  GBData,
  /// 吞吐量传感器 (单位: 字节/秒, 显示: "{value} B/s")
  Throughput,
  /// 数据速率传感器 (单位: 字节/秒, 显示: "{value} B/s")
  DataRate,
  /// 小数据包传感器 (单位: 字节, 显示: "{value} B")
  SmallData,
  /// 小数据包传感器 (单位: 字节, 显示: "{value} GB")
  GBSmallData,
  /// 前端总线传感器 (单位: 兆赫兹, 显示: "{value} MHz")
  FSB,
  /// 多路复用器传感器 (单位: 兆赫兹, 显示: "{value} MHz")
  Multiplexer,
  /// 平均时钟速度传感器 (单位: 兆赫兹, 显示: "{value} MHz")
  ClockAverage,
  /// 未知
  Unknown,
}
```
-----------------------------------------------------------
--args {HardwareType} {SensorType}
  1.HardwareType (硬件类型)默认所有
  2.SensorType（传感器类型）默认所有
--api:  `OHM` `AIDA64` `OS`(sysinfo Rust)
  1.OHM（https://openhardwaremonitor.org/） OpenHardwareMonitor开源硬件监控支持windows
  2.AIDA64（OpenHardwareMonitor） 只支持windows
  3.OS（use rust lib of sysinfo） Rust中sysinfo库支持跨平台
--task: `print` `check` `data`
  1.print（just print data）
  2.check（check value）
  3.data（return data）
-- {次数} {测试值} {误差范围+-} {负载0~100只支持CPU Clock}

# Example
-----------------------------------------------------------
# Cmd Example OHM
```sh
  # 打印所有的OHM接口的数据
hw --api OHM --task data
  # 测试OHM的CPU主频 5次 允许1000~5000的范围值 负载100
hw --api OHM --task check --args CPU Clock -- 5 3000 2000 100
  # 测试OHM的风扇转速 5次 允许1000~5000的范围值 不支持负载
hw --api OHM --task check --args ALL Fan -- 5 3000 2000
```

# Cmd Example AIDA64
```sh
  # 打印所有的AIDA64接口的数据
hw --api AIDA64 --task data
  # 测试AIDA64的CPU主频 5次 允许1000~5000的范围值 负载100
hw --api AIDA64 --task check --args CPU Clock -- 5 3000 2000 100
  # 测试AIDA64的风扇转速 5次 允许1000~5000的范围值 不支持负载
hw --api AIDA64 --task check --args ALL Fan -- 5 3000 2000
```
-----------------------------------------------------------

"#)]
#[allow(clippy::struct_excessive_bools)]
pub struct Opts {
  /// API接口
  #[structopt(required = true, short, long, possible_values = &OptsApi::variants(), case_insensitive = true)]
  pub api: OptsApi,
  /// 任务
  #[structopt(long, required = false, default_value = "")]
  pub task: String,
  /// 是否完整信息
  #[structopt(long)]
  pub full: bool,
  /// 筛选排除
  #[structopt(long, required = false)]
  pub filter: Vec<String>,
  /// 扩展参数
  #[structopt(long, required = false)]
  pub args: Vec<String>,
  /// 扩展指令
  #[structopt(required = false, last = true)]
  pub command: Vec<String>,
}
impl Default for Opts {
  fn default() -> Self {
    Self {
      api: OptsApi::OHM,
      task: String::new(),
      full: false,
      // verbose: 0,
      args: Vec::new(),
      filter: Vec::new(),
      command: Vec::new(),
    }
  }
}

impl Opts {
  /// # Example
  pub fn new<I>(args: Option<I>) -> AnyResult<Self>
  where
    Self: Sized,
    I: IntoIterator,
    I::Item: Into<OsString> + Clone,
  {
    match args {
      Some(arg) => match Opts::from_iter_safe(arg) {
        Ok(opt) => Ok(opt),
        Err(e) => Err(e.into()),
      },
      None => Ok(Opts::from_args()),
    }
  }
  /// # 检查空
  pub fn check_empty() -> bool {
    std::env::args().len() == 1
  }
}
