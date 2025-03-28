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
        CoreTemp,
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

### [1. 📖 点击Rust调用CLI](examples/cli.rs)
### [2. 📖 点击Rust调用OHM 获取CPU主频](examples/ohm_cpu_clock.rs)
### OpenHardwareMonitor 监控
![OHM监控界面](assets/screen/OHM.png)
**CPU Clock监控示例**

1. **data命令** - 仅返回当前值
```bash
hw --api OS --task data --args CPU Clock
```

```bash
# CPU温度监控
hw --api OHM --task check --args CPU Temperature

# CPU频率测试 (5次, 目标3000MHz, 误差±2000MHz, 100%负载)
hw --api OHM --task check --args CPU Clock -- 5 3000 2000 100

# 风扇转速测试 (5次, 目标3000RPM, 误差±2000RPM)
hw --api OHM --task check --args ALL Fan -- 5 3000 2000
```

### [3.📖 点击Rust调用OS 获取CPU主频](examples/os_cpu_clock.rs)
### sysinfo 监控
![系统监控界面](assets/screen/OS.png)
```bash
# 系统整体状态
hw --api OS --task print

# CPU负载监控
hw --api OS --task check --args CPU Load
```

### [4.📖 点击Rust调用AIDA64 获取CPU主频](examples/aida64_cpu_voltage.rs)
### AIDA64 监控
![AIDA64监控界面](assets/screen/AIDA64.png)
```bash
# 内存使用率监控
hw --api AIDA64 --task check --args RAM Load

# CPU核心电压监控
hw --api AIDA64 --task check --args CPU Voltage
```
### [5. 📖 点击Rust调用OSMore](examples/os_more_base.rs)
```bash
# 获取系统完整信息
hw --api OSMore --task OsFullVersion 
# 获取内存大小
hw --api OSMore --task MemoryTotal 
# 获取计算机名
hw --api OSMore --task HostName
# 获取OS版本
hw --api OSMore --task OsVersion
```
### [6. 📖 点击Rust调用微软OFFICE](examples/os_office.rs)
```bash
# 获取Office版本
hw --api OSOffice --task check-with-cache --args V2016 test
```
### [7. 📖 点击Rust调用微软系统激活](examples/os_system.rs)
```bash
# 激活系统
hw --api OSSystem --task active --args XXXXX-XXXXX-XXXXX-XXXXX-XXXXX activation_temp
# 检查系统激活状态并查询激活码缓存
hw --api OSSystem --task check-with-cache --args activation_temp
```
### [8. 📖 点击Rust调用导出DLL|SO动态链接库](examples/file_info.rs)
```bash
# 导出DLL|SO动态链接库
hw --api FileInfo --task copy-lib --args target/debug/hw.exe target/debug/_libs
# 打印文件节点
hw --api FileInfo --task print --args target/debug/hw.exe
# 打印文件节点
hw --api FileInfo --task nodes --args target/debug/hw.exe
```
### [9. 📖 点击Rust调用PING](examples/ping.rs)
```bash
# 测试PING
hw --api OSMore --task NetManage  --args ping 127.0.0.1 baidu.com 3
# 测试PING节点
hw --api OSMore --task NetManage --args ping-nodes baidu.com 3 -- ~is_connected Ethernet
```
### [10. 📖 点击Rust调用设置DHCP](examples/dhcp.rs)
```bash
# 设置DHCP ~is_connected 是指正在连接的网卡
hw --api OSMore --task NetManage --args dhcp -- ~is_connected
```
### [11. 📖 点击Rust调用设置静态IP](examples/static_ip.rs)
```bash
# 设置静态IP
hw --api OSMore --task NetManage  --args set-ip 192.168.1.100 255.255.255.0 192.168.1.1 -- "以太网"
# 设置DNS Ethernet=类型 "以太网"=名称   ~is_connected=网卡
hw --api OSMore --task NetManage  --args set-dns 223.5.5.5 114.114.114.114 "以太网" Ethernet  ~is_connected
```
### [12. 📖 点击Rust调用桌面](examples/desktop.rs)
```bash
# 桌面节点
hw --api OSMore --task Desktop --args nodes
# 打印
hw --api OSMore --task Desktop --args print
```
### [13. 📖 点击Rust调用驱动](examples/drive.rs)
```bash
# 扫描驱动
hw --api Drive --task scan
# 驱动打印
hw --api Drive --task print -- =net "*I225-V #6"
hw --api Drive --task print -- "@pci*" "*I225-V #6"
hw --api Drive --task print -- "@pci*" "PCI*" "*E0276CFFFFEEA86B00"
  # --full 完整数据 但更消耗资源，建议加=和@去筛选
hw --api Drive --task print --full -- =net "*I225-V #6" 
# 驱动节点
hw --api Drive --task nodes -- =net
# 导出驱动
hw --api Drive --task export --args oem6.inf D:\\drives
hw --api Drive --task export --args oem*.inf .
# 重启驱动
hw --api Drive --task restart -- =net "Intel(R) Ethernet Controller (3) I225-V #5"
hw --api Drive --task restart -- "@PCI\VEN_8086&DEV_15F3&SUBSYS_00008086&REV_03\E0276CFFFFEEA86A00"
# 启用驱动
hw --api Drive --task enable -- =net "Intel(R) Ethernet Controller (3) I225-V #5"
# 禁用驱动
hw --api Drive --task disable -- "@PCI\VEN_8086&DEV_15F3&SUBSYS_00008086&REV_03\E0276CFFFFEEA86A00"
# 删除驱动
hw --api Drive --task delete -- "@PCI\VEN_8086&DEV_15F3&SUBSYS_00008086&REV_03\E0276CFFFFEEA86A00"
# 增加驱动
hw --api Drive --task add  --args D:\\drives\\oem6.inf /install
# 增加驱动文件夹
hw --api Drive --task add-folder --args D:\\drives /install
# 检查驱动状态
hw --api Drive --task check-status
# 检查驱动状态并打印
hw --api Drive --task print-status
# 检查驱动状态并打印完整
hw --api Drive --task print-status --full
# 检查驱动状态并打印节点
hw --api Drive --task print-status --nodes
# 检查驱动状态并打印节点完整
hw --api Drive --task print-status --nodes --full
```
### [14. 📖 点击Rust调用同步时间](examples/sync_datetime.rs)
```bash
# 同步时间
hw --api OSMore --task NetManage --args sync-datetime time.windows.com
```
### [15. 📖 点击Rust调用网络接口](examples/net_interfaces.rs)
```bash
# "~Less100" 速度小于100
# "~100" 速度大于等于100
# "~1000" 速度大于等于1000
# "~Big1000" 速度大于等于10000
# "~is_connected" 正在连接
# "~has_dhcp_ip" 有DHCP IP

# 检查MAC重复和初始化
hw --api OSMore --task NetInterface --args check-mac "*I225-V #1" -- ~has_dhcp_ip
# 网络接口
hw --api OSMore --task NetInterface --args print  -- ~has_dhcp_ip
# 网络接口节点
hw --api OSMore --task NetInterface --args nodes  -- ~has_dhcp_ip
```
### [16. 📖 点击Rust调用磁盘](examples/disk.rs)
```bash
# 获取磁盘数据
hw --api Disk --task data --args C:
# 获取磁盘挂载树
hw --api Disk --task mount-tree --args C:
# 检查磁盘负载
hw --api Disk --task check-load --args 10 90
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
