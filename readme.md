<img src="assets/icon.ico" alt=""/>

### 📄 [中文](readme.zh.md)  | 📄  [English](readme.md)
[![Test Status](https://github.com/rust-random/rand/workflows/Tests/badge.svg?event=push)](https://github.com/eternalnight996/hw/actions) [![Book](https://img.shields.io/badge/book-master-yellow.svg)](https://doc.rust-lang.org/book/) [![API](https://img.shields.io/badge/api-master-yellow.svg)](https://github.com/eternalnight996/hw) [![API](https://docs.rs/hw/badge.svg)](https://docs.rs/rand)

# A Powerful Cross-Platform Hardware Monitoring Tool

## 📝 Project Introduction

**Integrates multiple hardware monitoring backends and provides a unified command-line interface**
This is a hardware monitoring tool written in Rust that supports multiple monitoring backends and sensor types. It can:

- Monitor system hardware status in real-time
- Support multiple hardware monitoring backends
  - OpenHardwareMonitor (Windows)
  - AIDA64 (Windows)
  - sysinfo (Cross-platform)
- Provide rich monitoring metrics
  - CPU (frequency, temperature, load, power)
  - GPU (NVIDIA/AMD graphics card status)
  - Memory usage
  - Hard drive status
  - Motherboard sensors
  - Fan speed
- Unified command-line interface
  - Simple and intuitive command parameters
  - Flexible data queries
  - Support data export
  - Threshold alerting functionality

## 💡 Main Features

- **Multi-backend Support**: Integrates various hardware monitoring solutions for different scenarios
- **Cross-platform Compatibility**: Provides basic cross-platform support through sysinfo
- **Rich Sensors**: Supports various sensor types including temperature, frequency, load, etc.
- **Real-time Monitoring**: Provides real-time hardware status monitoring and data collection
- **Unified Interface**: Simplified command-line interface with unified data format
- **Extensibility**: Modular design for easy extension of new monitoring backends
- **Performance Optimization**: Low resource usage with efficient data collection and processing

## 📸 Interface Preview and Command Examples

### OpenHardwareMonitor Monitoring
![OHM Monitoring Interface](assets/screen/OHM.png)

**CPU Clock Monitoring Example**

**Cargo Command Install Examples:**
```bash
cargo install hw
```
**just install examples:**
```bash
git clone https://github.com/eternalnight996/hw.git
cd hw
cargo install just
just
```

1. **data command** - Returns current value only
```bash
$ hw --api OS --task data --args CPU Clock
R<{"content":"2904","status":true,"opts":null}>R
```

2. **print command** - Returns complete statistics
```bash
$ hw --api OS --task print --args CPU Clock
R<{"content":"{\"api\":\"OS\",\"hw_type\":\"CPU\",\"sensor_type\":\"Clock\",\"res\":\"PASS\",\"data\":\"2904\",\"min\":2904.0,\"max\":2904.0,\"avg\":2904.0,\"total\":104544.0,\"samples\":36,\"test_secs\":0,\"error_count\":0,\"load\":{\"min\":0.0,\"max\":0.0,\"avg\":99.0,\"total\":3576.0,\"status\":[]},\"status\":[[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0]]}","status":true,"opts":null}>R
```

3. **check command** - Performs value range validation and load testing
```bash
$ hw --api OS --task check --args CPU Clock -- 10 2000 3000 100
R<{"content":"{\"api\":\"OS\",\"hw_type\":\"CPU\",\"sensor_type\":\"Clock\",\"res\":\"PASS\",\"data\":\"2904\",\"min\":2904.0,\"max\":2904.0,\"avg\":2904.0,\"total\":104544.0,\"samples\":36,\"test_secs\":0,\"error_count\":0,\"load\":{\"min\":0.0,\"max\":0.0,\"avg\":99.0,\"total\":3576.0,\"status\":[]},\"status\":[[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0],[\"\",2904.0]]}","status":true,"opts":null}>R
```

**Command Differences Explanation:**
- **data**: Returns current sensor value only
- **print**: Returns complete statistics without validation
- **check**: Performs value range validation and load testing
  - `10`: Number of tests
  - `2000`: Target value
  - `3000`: Error range (-1000~5000)
  - `100`: CPU load percentage

### AIDA64 Monitoring
![AIDA64 Monitoring Interface](assets/screen/AIDA64.png)

### sysinfo Monitoring
![System Monitoring Interface](assets/screen/OS.png)
### Sensor Type Support Details
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
    <td>Processor and memory clock frequencies</td>
  </tr>
  <tr>
    <td>Temperature</td>
    <td>°C</td>
    <td>{value} °C</td>
    <td>CPU, GPU, motherboard temperatures</td>
  </tr>
  <tr>
    <td>Load</td>
    <td>%</td>
    <td>{value}%</td>
    <td>Processor load, memory usage</td>
  </tr>
  <tr>
    <td>Fan</td>
    <td>RPM</td>
    <td>{value} RPM</td>
    <td>Fan speed</td>
  </tr>
  <tr>
    <td>Voltage</td>
    <td>V</td>
    <td>{value} V</td>
    <td>Various voltage values</td>
  </tr>
  <tr>
    <td>Power</td>
    <td>W</td>
    <td>{value} W</td>
    <td>Power consumption</td>
  </tr>
  <tr>
    <td>Flow</td>
    <td>L/h</td>
    <td>{value} L/h</td>
    <td>Liquid cooling flow rate</td>
  </tr>
  <tr>
    <td>Control</td>
    <td>%</td>
    <td>{value}%</td>
    <td>Fan control level</td>
  </tr>
  <tr>
    <td>Level</td>
    <td>%</td>
    <td>{value}%</td>
    <td>Battery level</td>
  </tr>
  <tr>
    <td>Data</td>
    <td>B</td>
    <td>{value} B</td>
    <td>Data size</td>
  </tr>
  <tr>
    <td>GBData</td>
    <td>GB</td>
    <td>{value} GB</td>
    <td>Large capacity data</td>
  </tr>
  <tr>
    <td>Throughput</td>
    <td>B/s</td>
    <td>{value} B/s</td>
    <td>Data throughput</td>
  </tr>
  <tr>
    <td>DataRate</td>
    <td>B/s</td>
    <td>{value} B/s</td>
    <td>Data transfer rate</td>
  </tr>
  <tr>
    <td>SmallData</td>
    <td>B</td>
    <td>{value} B</td>
    <td>Small data packets</td>
  </tr>
  <tr>
    <td>GBSmallData</td>
    <td>GB</td>
    <td>{value} GB</td>
    <td>Large capacity small data packets</td>
  </tr>
  <tr>
    <td>FSB</td>
    <td>MHz</td>
    <td>{value} MHz</td>
    <td>Front Side Bus frequency</td>
  </tr>
  <tr>
    <td>Multiplier</td>
    <td>MHz</td>
    <td>{value} MHz</td>
    <td>Clock multiplier</td>
  </tr>
  <tr>
    <td>ClockAverage</td>
    <td>MHz</td>
    <td>{value} MHz</td>
    <td>Average clock frequency</td>
  </tr>
</table>

### Hardware Type Support Details

<table>
  <tr>
    <th>Hardware Type</th>
    <th>Description</th>
    <th>Common Sensors</th>
  </tr>
  <tr>
    <td>CPU</td>
    <td>Central Processing Unit</td>
    <td>Clock, Temperature, Load, Power</td>
  </tr>
  <tr>
    <td>RAM</td>
    <td>Memory</td>
    <td>Data, Load, Clock</td>
  </tr>
  <tr>
    <td>Mainboard</td>
    <td>Motherboard</td>
    <td>Temperature, Voltage, Fan</td>
  </tr>
  <tr>
    <td>GpuNvidia</td>
    <td>NVIDIA Graphics Card</td>
    <td>Clock, Temperature, Load, Fan</td>
  </tr>
  <tr>
    <td>GpuAti</td>
    <td>AMD/ATI Graphics Card</td>
    <td>Clock, Temperature, Load, Fan</td>
  </tr>
  <tr>
    <td>HDD</td>
    <td>Hard Disk Drive</td>
    <td>Temperature, Load, Data</td>
  </tr>
  <tr>
    <td>SuperIO</td>
    <td>Super I/O Chip</td>
    <td>Fan, Temperature, Voltage</td>
  </tr>
  <tr>
    <td>TBalancer</td>
    <td>T-Balancer Device</td>
    <td>Fan, Flow, Temperature</td>
  </tr>
  <tr>
    <td>Heatmaster</td>
    <td>Heatmaster Device</td>
    <td>Fan, Flow, Temperature</td>
  </tr>
</table>

### Monitoring Backend Feature Comparison

<table>
  <tr>
    <th>Feature</th>
    <th>OHM</th>
    <th>AIDA64</th>
    <th>sysinfo</th>
  </tr>
  <tr>
    <td>Real-time Monitoring</td>
    <td><h4 style="color:green">✓</h4></td>
    <td><h4 style="color:green">✓</h4></td>
    <td><h4 style="color:green">✓</h4></td>
  </tr>
  <tr>
    <td>Historical Data</td>
    <td><h4 style="color:green">✓</h4></td>
    <td><h4 style="color:green">✓</h4></td>
    <td><h4 style="color:red">✗</h4></td>
  </tr>
  <tr>
    <td>Hardware Sensors</td>
    <td><h4 style="color:green">✓</h4></td>
    <td><h4 style="color:green">✓</h4></td>
    <td><h4 style="color:red">✗</h4></td>
  </tr>
  <tr>
    <td>Cross-platform Support</td>
    <td><h4 style="color:red">✗</h4></td>
    <td><h4 style="color:red">✗</h4></td>
    <td><h4 style="color:green">✓</h4></td>
  </tr>
</table>

## 🚀 Development Progress
<table>
  <tr>
    <th>Backend</th>
    <th>Windows</th>
    <th>Linux</th>
    <th>MacOS</th>
    <th>Status</th>
    <th>Description</th>
  </tr>
  <tr>
    <td>OpenHardwareMonitor</td>
    <td>
      <h4 style="color:green">✓</h4>
      <span style="color:#888">100%</span>
    </td>
    <td>
      <h4 style="color:orange">Not Supported</h4>
      <span style="color:#888">0%</span>
    </td>
    <td>
      <h4 style="color:orange">Not Supported</h4>
      <span style="color:#888">0%</span>
    </td>
    <td><h4 style="color:green">✓</h4><span>Completed</span></td>
    <td>Basic functionality complete, optimizing hardware compatibility</td>
  </tr>
  <tr>
    <td>AIDA64</td>
    <td>
      <h4 style="color:green">✓</h4>
      <span style="color:#888">100%</span>
    </td>
    <td>
      <h4 style="color:orange">Not Supported</h4>
      <span style="color:#888">0%</span>
    </td>
    <td>
      <h4 style="color:orange">Not Supported</h4>
      <span style="color:#888">0%</span>
    </td>
    <td><h4 style="color:green">✓</h4><span>Completed</span></td>
    <td>Main features stable, continuing to improve parsing</td>
  </tr>
  <tr>
    <td>sysinfo</td>
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
    <td><h4 style="color:blue">🔄</h4><span>Testing</span></td>
    <td>Cross-platform basic functions available, supporting more information retrieval</td>
  </tr>
</table>

> **Legend**:
> - ✓ Completed
> - ⚡ In Progress
> - 🔄 Beta/Testing
> - ✗ Not Started

> **Note**: 
> - OpenHardwareMonitor (OHM) and AIDA64 only support Windows platform
> - sysinfo supports cross-platform but with limited functionality
> - Specific sensor support may vary by hardware

## Quick Start

### Installation

```bash
# Install build tools
cargo install just
# More commands
just help
# Build project
just
```

### Basic Usage

```bash
# Print all hardware information
hw --api OHM --task print

# Check specific hardware metrics
hw --api OHM --task check --args CPU Temperature
```

## Command Line Parameters

```
hw --api <API> --task <TASK> --args <HW_TYPE> <SENSOR_TYPE> -- [OPTIONS]
```

### Parameter Description

- `--api`: Select monitoring backend
  - `OHM`: OpenHardwareMonitor
  - `AIDA64`: AIDA64
  - `OS`: sysinfo
- `--task`: Task type
  - `print`: Print data
  - `check`: Check values
  - `data`: Return raw data
- `--args`: Hardware and sensor type
- `--`: Additional parameters (test count/target value/error range/CPU load)

---
## Binary Call Usage Examples
### OpenHardwareMonitor
```bash
# CPU temperature monitoring
hw --api OHM --task check --args CPU Temperature

# CPU frequency test (5 times, target 3000MHz, error ±2000MHz, 100% load)
hw --api OHM --task check --args CPU Clock -- 5 3000 2000 100

# Fan speed test (5 times, target 3000RPM, error ±2000RPM)
hw --api OHM --task check --args ALL Fan -- 5 3000 2000
```

### AIDA64
```bash
# Memory usage monitoring
hw --api AIDA64 --task check --args RAM Load

# CPU core voltage monitoring
hw --api AIDA64 --task check --args CPU Voltage
```

### sysinfo
```bash
# Overall system status
hw --api OS --task print

# CPU load monitoring
hw --api OS --task check --args CPU Load
```

### OS More
```bash
# CPU name
hw --api OSMore --task CpuName
# Memory total
hw --api OSMore --task MemoryTotal
# ...
```
---
## Rust Call Usage Examples
### 📖 Features
```toml
[dependencies]
# All features
hw = {version="0.1", default-features = false, feature=["cli", "ohm", "aida64", "os"]}
# For packaging
hw = {version="0.1", default-features = false, feature=["cli", "ohm", "aida64", "os","build"]}
```

### 🔢 Using CLI for Internal Calls
```rust
#[cfg(feature = "cli")]
#[tokio::main]
async fn main() -> e_utils::AnyResult<()> {
  use e_utils::cmd::CmdResult;
  use hw::cli::api;
  use hw::cli::Opts;
  use serde_json::Value;
  let opts = Opts::new(None as Option<Vec<&str>>)?;
  let mut res: CmdResult<Value> = CmdResult {
    content: String::new(),
    status: false,
    opts: Value::Null,
  };
  match api(opts, &mut res.opts).await {
    Ok(v) => {
      res.content = v;
      res.status = true;
    }
    Err(e) => res.content = e.to_string(),
  }
  println!("\n{}", res.to_str()?);
  Ok(())
}
```

### [📖 Rust调用OHM做内部调用](./examples/ohm_cpu_clock.rs)
### [📖 Rust调用OS做内部调用](./examples/os_cpu_clock.rs)
### [📖 Rust调用AIDA64做内部调用](./examples/aida64_cpu_voltage.rs)
### [📖 Rust调用OS2做内部调用](./examples/os2_more.rs)
---
## Dependencies Version

- OpenHardwareMonitor: v0.9.6
- AIDA64: v7.40.7100
- sysinfo: v0.33

### Notes About Third-party Applications
When using OHM or AIDA64 interfaces, the program first checks if the process exists;
If not, it checks if `OpenHardwareMonitor.exe` or `aida64.exe` exists in the current directory

## 📊 Performance Benchmarks
---
## 🦊 Projects Using This Tool
`AUTOTEST2.exe`

---
## 🔭 Why Choose This Tool?

In the field of hardware monitoring, we often face these challenges:
- Large differences in monitoring interfaces across platforms
- Complex sensor data acquisition on Windows
- Rust support
- Lack of unified data access methods
- Cumbersome switching between multiple monitoring tools
- Limited automated testing support

This tool aims to solve these problems by providing:

### 🎯 Unified Access Interface
- **Command Line Tool**: Simple and intuitive CLI commands
- **Rust API**: Native Rust programming interface
- **WMI Support**: WMI query capability for Windows platform
- **Rust Support**: Direct library calls in Rust
- **Unified Data Format**: Standardized data output

### 💻 Seamless Multi-platform Support
- **Windows**: Complete sensor support (OHM/AIDA64)
- **Linux**: Basic system information monitoring (sysinfo)
- **MacOS**: Basic system information monitoring (sysinfo)

### 🔌 Rich Integration Capabilities
- **Automated Testing**: Support for automated hardware testing scenarios
- **Data Collection**: Flexible data collection and export
- **Monitoring Alerts**: Configurable threshold monitoring
- **Extension Interface**: Support for custom monitoring backends

### 🛠️ Ready to Use
- **Zero Configuration**: Minimal configuration requirements
- **Quick Deployment**: Single executable file
- **Backward Compatibility**: Maintains API stability
- **Complete Documentation**: Detailed usage instructions

### 📊 Typical Application Scenarios

1. **Hardware Testing**
   - Product quality validation
   - Performance benchmarking
   - Stability testing

2. **System Monitoring**
   - Server status monitoring
   - Workstation performance analysis
   - Temperature control system monitoring

3. **Development Debugging**
   - Hardware driver development
   - Performance optimization analysis
   - Problem diagnosis

4. **Automation Integration**
   - CI/CD pipeline integration
   - Automated test scripts
   - Monitoring system integration

> 💡 **Design Philosophy**: 
> - Simplicity first
> - Unified interface standards
> - Cross-platform compatibility
> - Extensible architecture

---
## 🙋 Reference Projects and Resources
- [Open Hardware Monitor Official Documentation](https://openhardwaremonitor.org/)
- [AIDA64 Official Documentation](https://www.aida64.com/)
- [sysinfo Crates Official](https://crates.io/crates/sysinfo)

---
## License

[LICENSE](LICENSE)
[COPYRIGHT](COPYRIGHT)