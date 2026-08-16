<div align="center">
  <img src="assets/icon.ico" alt="HW Monitor" width="120"/>
  <h1>HW Monitor</h1>
  <p><strong>强大而统一的跨平台硬件监控工具</strong></p>
</div>

<div align="center">
  
[![API](https://img.shields.io/badge/api-master-yellow.svg)](https://github.com/eternalnight996/hw)[![API](https://docs.rs/e-log/badge.svg)](https://docs.rs/hw)[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

[![CI](https://github.com/eternalnight996/hw/actions/workflows/ci.yml/badge.svg)](https://github.com/eternalnight996/hw/actions/workflows/ci.yml)

[English](readme.md) | 简体中文

</div>

## ✨ 特性一览

- 🔄 **多后端集成** - 支持 OpenHardwareMonitor、AIDA64、sysinfo 等多种监控方案
- 🌍 **跨平台支持** - 完整支持 Windows，基础支持 Linux/MacOS
- 📊 **丰富的监控指标** - CPU、GPU、内存、硬盘、主板等全方位监控
- ⚡ **实时数据采集** - 毫秒级的硬件状态监控
- 🔌 **统一接口** - 简洁的命令行工具与 Rust API
- 🛠 **可扩展架构** - 轻松扩展新的监控后端
- 📈 **性能优化** - 低资源占用，高效数据处理

## 🚀 快速开始

### 通过 Cargo 安装
```bash
cargo install hw
```

### 从源码构建
```bash
git clone https://github.com/eternalnight996/hw.git
cd hw
cargo install just
just
```

> 运行日志以 e-log（tracing）方式输出：按天滚动写入 `logs/hw-*.log`（含时间戳+级别），控制台输出到 **stderr**；stdout 仅保留 `R<...>R` 协议行，etest 解析不受日志干扰。

> CI（GitHub Actions）覆盖：`cargo check --all-features`、`--no-default-features --features "cli,log"`、`--features "ohm,cli,log"` 与 `cargo test`（lib + doc）。

**默认启动为 GUI**：`cargo run`（或 `target\\debug\\hw-gui.exe`）打开桌面应用，含「实时监控 / Check 测试 / etest 规则」三个视图；命令行工具仍为 `hw`（如 `hw --api Test --task list`）。

**命令区别说明：**
- **data**: 仅返回传感器当前值
- **print**: 返回完整统计信息，但不做验证
- **check**: 进行值范围验证和负载测试
  - `10`: 测试次数
  - `2000`: 目标值
  - `3000`: 误差范围 (-1000~5000)
  - `100`: CPU负载百分比

---
### 📖 特性
```toml
[dependencies]
# 所有特性
hw = {version="0.1"}
# 打包所有特性
hw = {version="0.1",feature=["build","built"]}
# 只用OHM
hw = {version="0.1", default-features = false, feature=["ohm"]}
# 命令行则加上cli
# 日志 支持 log 和 tracing, cli则默认支持println输出
hw = {version="0.1", default-features = false, feature=["ohm","cli","log"]}
```

---
## 📸 界面预览与命令示例

### [1. 📖 点击Rust调用CLI](examples/cli.rs)
### [2. 📖 点击Rust调用OHM 获取CPU主频](examples/ohm_cpu_clock.rs)
### OpenHardwareMonitor 监控
![OHM监控界面](assets/screen/OHM.png)
**CPU Clock监控示例**

1. **data命令** - 仅返回当前值
```bash
hw --api OS --task data --args CPU Clock
```
```text
   Compiling hw v0.1.2 (D:\MyApp\hw)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.18s
     Running `target\x86_64-pc-windows-msvc\debug\hw.exe --api OHM --task data --args CPU Clock`
Started OpenHardwareMonitor.exe with PID: 5332
Loading... (100%/100%)
...
--------------------------------
平均值（1068MHz  0.0%）   数据:1068

关闭负载

=== 总结 -> CPU 中央处理器 ===
--- 传感器 -> Clock 频率 MHz ---
结果: PASS
数据: 1068
目标: 0.0 MHz
平均: 1068.0 MHz
最低: 901.2 MHz
最高: 1101.5 MHz
次数: 1
错误次数: 0
负载: 0.0%
平均负载: 0.0%
允许误差: ±0.0
允许范围: 0.0 ~ 0.0 MHz
====================


R<{"content":"1068","status":true,"opts":null}>R
```

2. **print命令** - 返回完整统计信息
```bash
hw --api OHM --task print --full --args CPU Clock
```
```text
...

R<{"content":"{\"api\":\"OHM\",\"hw_type\":\"CPU\",\"sensor_type\":\"Clock\",\"res\":\"PASS\",\"data\":\"1102\",\"min\":1101.5174560546875,\"max\":1101.5174560546875,\"avg\":1102.0,\"total\":6609.104736328125,\"samples\":6,\"test_secs\":0,\"error_count\":0,\"load\":{\"min\":0.0,\"max\":0.0,\"avg\":0.0,\"total\":0.0,\"status\":[]},\"status\":[...]}","status":true,"opts":null}>R
```

3. **check命令** - 进行值范围验证和负载测试
```bash
hw --api OHM --task check --full --args CPU Clock -- 10 2000 3000 100
```
```text
...

--- 第 10 秒中央处理器状态 ---
CPU Core #1 - 当前=2904.0 MHz(频率) 误差: ±3000.0
CPU Core #6 - 当前=2904.0 MHz(频率) 误差: ±3000.0
CPU Core #5 - 当前=2904.0 MHz(频率) 误差: ±3000.0
CPU Core #4 - 当前=2904.0 MHz(频率) 误差: ±3000.0
CPU Core #3 - 当前=2904.0 MHz(频率) 误差: ±3000.0
CPU Core #2 - 当前=2904.0 MHz(频率) 误差: ±3000.0
--------------------------------
平均值（2904MHz  99.0%）   数据:2904

关闭负载

=== 总结 -> CPU 中央处理器 ===
--- 传感器 -> Clock 频率 MHz ---
结果: PASS
数据: 2904
目标: 2000.0 MHz
平均: 2904.0 MHz
最低: 2904.0 MHz
最高: 2904.0 MHz
次数: 10
错误次数: 0
负载: 100.0%
平均负载: 99.0%
允许误差: ±3000.0
允许范围: -1000.0 ~ 5000.0 MHz
====================


R<{"content":"{\"api\":\"OHM\",\"hw_type\":\"CPU\",\"sensor_type\":\"Clock\",\"res\":\"PASS\",\"data\":\"2904\",\"min\":2904.000732421875,\"max\":2904.001708984375,\"avg\":2904.0,\"total\":174240.07470703125,\"samples\":60,\"test_secs\":0,\"error_count\":0,\"load\":{\"min\":0.0,\"max\":0.0,\"avg\":99.0,\"total\":5946.0,\"status\":[]},\"status\":[...]}","status":true,"opts":null}>R 
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

### [X.📖 Click for Rust CoreTemp CPU Voltage Usage](examples/core_temp.rs)
### CoreTemp Monitoring
![CoreTemp Monitor Interface](assets/screen/CoreTemp.png)
```bash
# Memory Usage Monitoring
hw --api CoreTemp --task check --args CPU Temperature

# CPU Core Voltage Monitoring
hw --api CoreTemp --task check --args CPU Clock
```

### [X.📖 Click for Rust LibreHardwareMonitor CPU Voltage Usage](examples/lhm_cpu_clock.rs)
```bash
# CPU温度监控
hw --api LHM --task check --args CPU Temperature

# CPU频率测试 (5次, 目标3000MHz, 误差±2000MHz, 100%负载)
hw --api LHM --task check --args CPU Clock -- 5 3000 2000 100

# 风扇转速测试 (5次, 目标3000RPM, 误差±2000RPM)
hw --api LHM --task check --args ALL Fan -- 5 3000 2000
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
---
### [17. 📖 点击Rust调用测试模式](src/test_mode/)
### 测试模式（可注册测试框架，受 TrafficMonitor 插件接口启发）
```bash
# 列出所有已注册模式
hw --api Test --task list

# 网速上/下行速率（B/s），5 次采样
hw --api Test --task net-speed --args print -- 5

# CPU 利用率检查：5 秒，目标 80%，误差 ±10%，并施加 60% 负载
hw --api Test --task cpu-usage --args check --filter CPU_Usage_Global -- 5 80 10 60

# 内存利用率，单次数据
hw --api Test --task mem-usage --args data

# 磁盘占用率与 IO 速率
hw --api Test --task disk-usage --args print -- 3

# 温度（°C）与显卡利用率（需在 plugins/ 放置 OHM/LHM/AIDA64）
hw --api Test --task temp --args print -- 3
hw --api Test --task gpu-usage --args check --filter "GPU Core" -- 5 90 10
```
动词为 `--args` 首参数（`data` / `print` / `check`，缺省 `print`）；测试参数跟在 `--` 后：`<秒数> <目标值> <误差> <负载%>`；`--filter` 限制参与校验/输出的指标。

**新增一个测试模式 = 1 个模块 + 1 行注册**（无需改动分发代码）：
1. 实现 `TestMode`（name/description/create）+ `ModeInstance`（sample，可选 setup/teardown/spawn_load）
2. 声明 `pub static MODE: XxxMode = XxxMode;`
3. 在 `src/test_mode/builtin/mod.rs` 注册一行

详见 [src/test_mode/mod.rs](src/test_mode/mod.rs) 的 trait 文档。
---
### [18. 📖 图形化界面 — hw-gui（eframe/egui 桌面应用）](src/bin/hw-gui.rs)
基于 eframe/egui 的桌面 GUI（`hw-gui`），形态参考 TrafficMonitor 的悬浮窗风格：

- **左侧栏 — 测试功能项**：`hw-config.json` plan 的全部功能项，每项带「是否测试」勾选框（解锁后可改并保存）与上次 PASS/FAIL/跳过 状态；下方为实时指标值
- **中间 — 测试数据线图**（固定布局，测试时更新）：当前规则样本曲线 / 实时曲线 / Check 曲线（含目标带）
- **Check 与规则运行**在顶栏操作；可导出与 CLI 相同的 etest 报告 JSON（见第 19 节）

```bash
# 构建（gui 特性引入 eframe/egui_plot；需要 rustc >= 1.95）
cargo build --features gui --bin hw-gui

# 运行
target\debug\hw-gui.exe

# 自动化冒烟测试：3 秒后自动关闭窗口
set HW_GUI_SMOKE=1 && target\debug\hw-gui.exe
```

温度/GPU 模式需要 `plugins/` 下有 OHM/LHM/AIDA64（见第 17 节），首次启动这些模式最长约 20 秒（拉起后端进程）；其余模式开箱即用。
---
### [19. 📖 etest 测试平台接入](src/test_mode/rules.rs)
生产测试由 **etest** 平台调度：平台通过命令行调用 `hw.exe`，解析标准输出的 `R<...>R` 包装 JSON：

```text
R<{"content":"...","status":true,"opts":null}>R
```

> **输出契约**：明细（逐秒进度/汇总）经 e-log 正常输出（`logs/hw-*.log` + stderr），不含 `R<...>R`；测试结束时，将全项产测明细以 `R<...>R` 作为**结果日志（`gui.log_file`，默认 `hw-gui-test.log`）的最后一行**追加 —— `status` = 整体 PASS/FAIL，`content` = 规则报告 JSON。

> 规则/配置格式参考兄弟项目 **MVCheck**（机内视觉检查上位机，`Conf.json` 模式）：随仓库提供模板文件、首次运行自动生成、etest 直接编辑。

**测试规则 —— 统一配置 `hw-config.json` 的 `plan` 段**（etest 只改这一个文件：`gui`=运行行为，`plan`=测试规则）。每个测试项一条，含上下限与稳定性：

| 字段 | 说明 | 示例 |
| --- | --- | --- |
| `id` | 测试项 ID（唯一） | `net-up` |
| `description` | 中文描述（GUI 与报告直接显示，如 `CPU 主频（MHz）`） | `CPU 主频（MHz）` |
| `enabled` | **是否测试该项**：`true` = 执行并判定；`false` = 跳过（仍显示在清单，报告标记 `skipped`，不参与整体判定） | `true` |
| `mode` | 已注册测试模式（见第 17 节） | `net-speed` |
| `metric` | 指标名包含匹配；空 = 该模式全部指标须通过 | `Total_Rx` |
| `unit` | 可选单位校验 | `B/s` |
| `min` / `max` | 上下限，按采样**平均值**判定；省略 = 不限 | `1000000` / `null` |
| `max_std` | **稳定性**：采样标准差 σ 上限；省略 = 不限 | `2.0` |
| `secs` | 采样秒数（默认 3） | `5` |
| `load` | 负载%（默认 0） | `0` |

`plan` 段模板（入库的 `hw-config.json` 含 `gui` + 本 `plan`）：

```json
{
  "name": "my-plan",
  "rules": [
    { "id": "net-up", "mode": "net-speed", "metric": "Total_Rx", "unit": "B/s", "min": 1000000, "max": null, "max_std": null, "secs": 5, "load": 0 },
    { "id": "ram-usage", "mode": "mem-usage", "metric": "RAM_Usage", "unit": "%", "min": null, "max": 90, "max_std": 2.0, "secs": 3, "load": 0 }
  ]
}
```

**命令：**

```bash
# 生成/刷新规则模板
# 重新生成统一配置模板（gui + plan）
hw --api Test --task config-template --args hw-config.json

# 执行 hw-config.json 内的 plan（也兼容裸规则文件）
hw --api Test --task run-rules --args hw-config.json
```

`content` 中的报告 JSON（逐项）：`{plan, status, results:[{item, mode, metric, unit, value, avg, min, max, std_dev, samples, min_limit, max_limit, max_std_limit, pass, message}]}`。拿到官方 etest 规范后可按其字段名对齐。

GUI 的「etest 规则」视图可直接加载同一规则文件逐条执行（带进度与曲线），并导出与 CLI 完全一致的报告 JSON —— 最终生产测试可以全程在 GUI 中运行。

**全功能项对照表**（限值为建议值，按你的产品调整）：

| 功能项 | mode | metric（建议） | 建议上限/说明 |
| --- | --- | --- | --- |
| CPU 主频 | `cpu-clock` | 任意（空=全部核心） | MHz，稳定性可加 `max_std` |
| CPU 温度 | `temp` | `CPU Package` | ≤ 85 °C |
| GPU 温度 | `temp` | `GPU` | ≤ 90 °C |
| 主板温度 | `temp` | `Mainboard` | ≤ 60 °C |
| 风扇转速 | `fan-speed` | 任意（空=全部风扇） | 建议 `min` ≥ 500 RPM |
| 电压 | `voltage` | 任意 | V，按规格填 min/max |
| 功率 | `power` | 任意 | W，按规格填 max |
| CPU 利用率 | `cpu-usage` | `CPU_Usage_Global` | % |
| 内存利用率 | `mem-usage` | `RAM_Usage` | ≤ 90 %，稳定性 `max_std` |
| 磁盘占用 | `disk-usage` | `C: Used%` | ≤ 90 % |
| 网速 | `net-speed` | `Total_Rx` / `Total_Tx` | B/s，按需 `min` |
| GPU 利用率 | `gpu-usage` | 任意 | % |

---
### [20. 📖 统一配置表（`hw-config.json`，etest 只改这一个文件）](src/gui_config.rs)
etest / 操作员直接编辑**唯一文件 `hw-config.json`**（首次运行自动生成）—— `gui` 段控制运行行为、`plan` 段为测试规则，无需改代码：

| 字段 | 含义 | 默认值 |
| --- | --- | --- |
| `lock` | 配置锁：`{enabled, password}` —— `enabled=true` 时 GUI 所有配置项只读（防误改）；解锁需密码（默认 `admin`，仅本会话生效） | `{true, "admin"}` |
| `default_view` | 启动视图：`live` / `check` / `rules` | `rules` |
| `auto_run` | 启动后自动开始执行规则 | `true` |
| `run_seconds` | 测试总时长上限（秒）；`0` = 不限（按每条规则自身 secs），超时后剩余规则标记超时 | `0` |
| `auto_close` | 测试完成后自动关闭窗口 | `true` |
| `exit_code_on_fail` | 测试失败时进程退出码返回 `1`（etest 不解析也能判断；仅 auto_close 时生效） | `true` |
| `raise_load_percent` | 全局负载%；>0 时作为未指定负载规则的默认负载（也是 Check 默认负载） | `0` |
| `display_mode` | `all` = 显示全部指标；`single` = 只显示 `display_metrics` 指定指标 | `all` |
| `display_metrics` | `display_mode=single` 时按指标名包含匹配显示（如 `["CPU_0_Clock"]` 只看 CPU 主频、`["CPU_Usage_Global"]` 只看占用） | `[]` |
| `check_params` | Check 测试参数（etest 可直接修改）：`{secs, target, error, load}`（秒数/目标值/±误差/负载） | `{5, 1000, 500, 0}` |
| `log_file` | 测试结果日志文件，`R<...>R` 结果追加写入（支持 `{origin}`/`{env:KEY}`，空 = 不写文件） | `hw-gui-test.log` |

产线一键示例：启动即进规则视图 → 自动运行 `plan` → 拉 60% 负载 → 只看 CPU 主频/占用 → 完成自动关闭并以退出码上报：

```json
{
  "gui": {
    "default_view": "rules",
    "auto_run": true,
    "run_seconds": 60,
    "auto_close": true,
    "exit_code_on_fail": true,
    "raise_load_percent": 60,
    "display_mode": "single",
    "display_metrics": ["CPU_0_Clock", "CPU_Usage_Global"],
    "check_params": { "secs": 5, "target": 1000, "error": 500, "load": 0 },
    "log_file": "hw-gui-test.log"
  },
  "plan": { "name": "全项产测", "rules": [ ... 12 项 ... ] }
}
```
---
## 🚀 开发进度
<table>
  <tr>
    <th>Backend</th>
    <th>Windows</th>
    <th>Linux</th>
    <th>MacOS</th>
    <th>Status</th>
    <th>Description</th>
    <th>支持功能</th>
  </tr>
  <tr>
    <td>CoreTemp</td>
    <td>
      <h4 style="color:green">✓</h4>
      <span style="color:#888">100%</span>
    </td>
    <td>
      <h4>-</h4>
      <span style="color:#888">0%</span>
    </td>
    <td>
      <h4>-</h4>
      <span style="color:#888">0%</span>
    </td>
    <td><h4 style="color:green">✓</h4><span>已完成</span></td>
    <td>获取硬件传感器数据<br>完成所有功能开发</td>
    <td>HardwareType(硬件类型),SensorType(传感器类型)</td>
  </tr>
  <tr>
    <td>OHM</td>
    <td>
      <h4 style="color:green">✓</h4>
      <span style="color:#888">100%</span>
    </td>
    <td>
      <h4>-</h4>
      <span style="color:#888">0%</span>
    </td>
    <td>
      <h4>-</h4>
      <span style="color:#888">0%</span>
    </td>
    <td><h4 style="color:green">✓</h4><span>已完成</span></td>
    <td>获取硬件传感器数据<br>完成所有功能开发</td>
    <td>HardwareType(硬件类型),SensorType(传感器类型)</td>
  </tr>
  <tr>
    <td>AIDA64</td>
    <td>
      <h4 style="color:green">✓</h4>
      <span style="color:#888">100%</span>
    </td>
    <td>
      <h4>-</h4>
      <span style="color:#888">0%</span>
    </td>
    <td>
      <h4>-</h4>
      <span style="color:#888">0%</span>
    </td>
    <td><h4 style="color:green">✓</h4><span>已完成</span></td>
    <td>获取硬件传感器数据<br>完成所有功能开发</td>
    <td>HardwareType(硬件类型),SensorType(传感器类型)</td>
  </tr>
  <tr>
    <td>OS</td>
    <td>
      <h4 style="color:green">⚡</h4>
      <span style="color:#888">10%</span>
    </td>
    <td>
      <h4 style="color:green">⚡</h4>
      <span style="color:#888">10%</span>
    </td>
    <td>
      <h4 style="color:green">⚡</h4>
      <span style="color:#888">10%</span>
    </td>
    <td><h4 style="color:blue">🔄</h4><span>测试中</span></td>
    <td>接口Rust system跨平台基础功能可用<br>支持更多的信息获取</td>
    <td>CPU,RAM</td>
  </tr>
  <tr>
    <td>OSMore</td>
    <td>
      <h4 style="color:green">⚡</h4>
      <span style="color:#888">70%</span>
    </td>
    <td>
      <h4 style="color:green">⚡</h4>
      <span style="color:#888">70%</span>
    </td>
    <td>
      <h4 style="color:green">⚡</h4>
      <span style="color:#888">70%</span>
    </td>
    <td><h4 style="color:green">✓</h4><span>已完成</span></td>
    <td>主要用于获取更多信息和管理</td>
    <td>MemoryTotal(内存大小),CpuCoreCount(CPU内核数量),OsVersion(系统版本)<br>OsFullVersion(系统版本),KernelVersion(内核版本),HostName(主机名),Uptime(开机时间)<br>CpuUsage(CPU使用率),MemoryUsage(内存使用率),CpuArch(CPU架构),UserNames(用户名),<br>NetInterface(网络接口),NetManage[网络管理(DHCP、PING、同步时间、静态IP配置)],Desktop(桌面),Drive(硬盘),</td>
  </tr>
  <tr>
    <td>Drive</td>
    <td>
      <h4 style="color:green">✓</h4>
      <span style="color:#888">100%</span>
    </td>
    <td>
      <h4 style="color:green">⚡</h4>
      <span style="color:#888">0%</span>
    </td>
    <td>
      <h4 style="color:green">⚡</h4>
      <span style="color:#888">0%</span>
    </td>
    <td><h4 style="color:green">✓</h4><span>已完成</span></td>
    <td>对接PNPUTIL和devcon</td>
    <td>scan(扫描),add-folder(添加文件),add(添加),delete(删除),delete-find(删除并查找),<br>print(打印),nodes(节点),restart(重启),enable(启用),disable(禁用),remove(移除),export(导出)</td>
  </tr>
  <tr>
    <td>FileInfo</td>
    <td>
      <h4 style="color:green">✓</h4>
      <span style="color:#888">100%</span>
    </td>
    <td>
      <h4 style="color:green">⚡</h4>
      <span style="color:#888">99%</span>
    </td>
    <td>
      <h4 style="color:green">⚡</h4>
      <span style="color:#888">99%</span>
    </td>
    <td><h4 style="color:green">✓</h4><span>已完成</span></td>
    <td>获取文件的依赖dll或so，支持一键导出依赖</td>
    <td>copy-lib(复制依赖),print(打印),nodes(列表)</td>
  </tr>
  <tr>
    <td>OSSystem</td>
    <td>
      <h4 style="color:green">✓</h4>
      <span style="color:#888">100%</span>
    </td>
    <td>
      <h4>-</h4>
      <span style="color:#888">0%</span>
    </td>
    <td>
      <h4>-</h4>
      <span style="color:#888">0%</span>
    </td>
    <td><h4 style="color:green">✓</h4><span>已完成</span></td>
    <td>系统</td>
    <td>check(检查),check-with-cache(检查并缓存),activate(激活),deactivate(注销),kms(注册kms),clear-kms(清理kms),clear-cache(清理缓存),cache-kms(缓存激活码)</td>
  </tr>
  <tr>
    <td>OSOffice</td>
    <td>
      <h4 style="color:green">✓</h4>
      <span style="color:#888">90%</span>
    </td>
    <td>
      <h4>-</h4>
      <span style="color:#888">0%</span>
    </td>
    <td>
      <h4>-</h4>
      <span style="color:#888">0%</span>
    </td>
    <td><h4 style="color:green">✓</h4><span>已完成</span></td>
    <td>Office</td>
    <td>check(检查),check-with-cache(检查并缓存),activate(激活),kms(注册kms),clear-kms(清理kms),clear-cache(清理缓存),cache-kms(缓存激活码)</td>
  </tr>
</table>

> **Note**: 
> - OpenHardwareMonitor (OHM) 和 AIDA64 仅支持 Windows 平台
> - sysinfo 支持跨平台但功能相对有限
> - 具体传感器支持可能因硬件而异


### 传感器类型支持详情
<table>
  <tr>
    <th>Sensor Type</th>
    <th>Unit</th>
    <th>Format</th>
    <th>Description</th>
  </tr>
  <tr>
    <td>Clock</td>
    <td>MHz</td>
    <td>{value} MHz</td>
    <td>处理器、内存等时钟频率</td>
  </tr>
  <tr>
    <td>Temperature</td>
    <td>°C</td>
    <td>{value} °C</td>
    <td>CPU、GPU、主板等温度</td>
  </tr>
  <tr>
    <td>Load</td>
    <td>%</td>
    <td>{value}%</td>
    <td>处理器负载、内存使用率</td>
  </tr>
  <tr>
    <td>Fan</td>
    <td>RPM</td>
    <td>{value} RPM</td>
    <td>风扇转速</td>
  </tr>
  <tr>
    <td>Voltage</td>
    <td>V</td>
    <td>{value} V</td>
    <td>各种电压值</td>
  </tr>
  <tr>
    <td>Power</td>
    <td>W</td>
    <td>{value} W</td>
    <td>功率消耗</td>
  </tr>
  <tr>
    <td>Flow</td>
    <td>L/h</td>
    <td>{value} L/h</td>
    <td>液体冷却流量</td>
  </tr>
  <tr>
    <td>Control</td>
    <td>%</td>
    <td>{value}%</td>
    <td>风扇控制等级</td>
  </tr>
  <tr>
    <td>Level</td>
    <td>%</td>
    <td>{value}%</td>
    <td>电池电量等级</td>
  </tr>
  <tr>
    <td>Data</td>
    <td>B</td>
    <td>{value} B</td>
    <td>数据大小</td>
  </tr>
  <tr>
    <td>GBData</td>
    <td>GB</td>
    <td>{value} GB</td>
    <td>大容量数据</td>
  </tr>
  <tr>
    <td>Throughput</td>
    <td>B/s</td>
    <td>{value} B/s</td>
    <td>数据吞吐量</td>
  </tr>
  <tr>
    <td>DataRate</td>
    <td>B/s</td>
    <td>{value} B/s</td>
    <td>数据传输速率</td>
  </tr>
  <tr>
    <td>SmallData</td>
    <td>B</td>
    <td>{value} B</td>
    <td>小数据��</td>
  </tr>
  <tr>
    <td>GBSmallData</td>
    <td>GB</td>
    <td>{value} GB</td>
    <td>大容量小数据包</td>
  </tr>
  <tr>
    <td>FSB</td>
    <td>MHz</td>
    <td>{value} MHz</td>
    <td>前端总线频率</td>
  </tr>
  <tr>
    <td>Multiplexer</td>
    <td>MHz</td>
    <td>{value} MHz</td>
    <td>倍频器</td>
  </tr>
  <tr>
    <td>ClockAverage</td>
    <td>MHz</td>
    <td>{value} MHz</td>
    <td>平均时钟频率</td>
  </tr>
</table>

### 硬件类型支持详情

<table>
  <tr>
    <th>Hardware Type</th>
    <th>Description</th>
    <th>Common Sensors</th>
  </tr>
  <tr>
    <td>CPU</td>
    <td>中央处理器</td>
    <td>Clock, Temperature, Load, Power</td>
  </tr>
  <tr>
    <td>RAM</td>
    <td>内存</td>
    <td>Data, Load, Clock</td>
  </tr>
  <tr>
    <td>Mainboard</td>
    <td>主板</td>
    <td>Temperature, Voltage, Fan</td>
  </tr>
  <tr>
    <td>GpuNvidia</td>
    <td>NVIDIA显卡</td>
    <td>Clock, Temperature, Load, Fan</td>
  </tr>
  <tr>
    <td>GpuAti</td>
    <td>AMD/ATI显卡</td>
    <td>Clock, Temperature, Load, Fan</td>
  </tr>
  <tr>
    <td>HDD</td>
    <td>硬盘驱动器</td>
    <td>Temperature, Load, Data</td>
  </tr>
  <tr>
    <td>SuperIO</td>
    <td>Super I/O芯片</td>
    <td>Fan, Temperature, Voltage</td>
  </tr>
  <tr>
    <td>TBalancer</td>
    <td>T-Balancer设备</td>
    <td>Fan, Flow, Temperature</td>
  </tr>
  <tr>
    <td>Heatmaster</td>
    <td>Heatmaster设备</td>
    <td>Fan, Flow, Temperature</td>
  </tr>
</table>


---
## 第三方接口版本
- OpenHardwareMonitor: v0.9.6
- AIDA64: v7.40.7100
- sysinfo: v0.33

### 关于第三方应用的说明
如果是用OHM 或 AIDA64接口，程序先会检查进程是否存在；
如果不存在则会检查当前目录是否存在 `OpenHardwareMonitor.exe` 或 `aida64.exe`

## 📊 性能基准
---
## 🦊 已运用项目
`AUTOTEST2.exe`

---
## 🔭 为什么选择这个工具？

在硬件监控领域，我们经常遇到以下挑战：
- 不同平台的监控接口差异大
- Windows下传感器数据获取复杂
- 支持Rust
- 缺乏统一的数据访问方式
- 多种监控工具切换繁琐
- 自动化测试支持有限

本工具致力于解决这些问题，提供：

### 🎯 统一的访问接口
- **命令行工具**: 简单直观的 CLI 命令
- **Rust API**: 原生的 Rust 编程接口
- **WMI 支持**: Windows 平台的 WMI 查询能力
- **Rust 支持**: Rust直接调用LIB
- **统一数据格式**: 标准化的数据输出

### 💻 多平台无缝支持
- **Windows**: 完整的传感器支持 (OHM/AIDA64)
- **Linux**: 基础系统信息监控 (sysinfo)
- **MacOS**: 基础系统信息监控 (sysinfo)

### 🔌 丰富的集成能力
- **自动化测试**: 支持自动化硬件测试场景
- **数据采集**: 灵活的数据收集和导出
- **监控告警**: 可配置的阈值监控
- **扩展接口**: 支持自定义监控后端

### 🛠️ 开箱即用
- **零配置**: 最小化配置需求
- **快速部署**: 单文件执行程序
- **向后兼容**: 保持 API 稳定性
- **完整文档**: 详细的使用说明

### 📊 典型应用场景

1. **硬件测试**
   - 产品质量验证
   - 性能基准测试
   - 稳定性测试

2. **系统监控**
   - 服务器状态监控
   - 工作站性能分析
   - 温控系统监测

3. **开发调试**
   - 硬件驱动开发
   - 性能优化分析
   - 问题诊断

4. **自动化集成**
   - CI/CD 管道集成
   - 自动化测试脚本
   - 监控系统对接

> 💡 **设计理念**: 
> - 简单易用优先
> - 统一接口标准
> - 跨平台兼容
> - 可扩展架构

---
## 🙋 参考项目与资料
- [Open Hardware Monitor 官方文档](https://openhardwaremonitor.org/)
- [AIDA64 官方文档](https://www.aida64.com)
- [sysinfo Crates官方](https://crates.io/crates/sysinfo)

---
## 许可证

[LICENSE](LICENSE)
[COPYRIGHT](COPYRIGHT)

## 🤝 参与贡献

我们欢迎任何形式的贡献！

- 提交 Issue 报告 bug 或提出新功能建议
- 提交 Pull Request 改进代码
- 完善项目文档
- 分享使用经验

在提交 PR 之前，请确保：
1. 代码符合项目规范
2. 添加必要的测试
3. 更新相关文档

## 📜 开源协议

本项目采用 [MIT](LICENSE-MIT) 和 [Apache 2.0](LICENSE-APACHE) 双重协议。

---

<div align="center">
  <sub>Built with ❤️ by eternalnight996 and contributors.</sub>
</div>