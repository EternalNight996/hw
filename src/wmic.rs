#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
use e_utils::AnyResult;
use serde::Deserialize;
#[derive(Debug, Deserialize)]
pub struct Hardware<T> {
    /// Combination of make and model (e.g., "Intel Core 2 Duo E8400")
    pub Name: String,
    /// Unique identifier for this hardware (e.g., "/intelcpu/0")
    pub Identifier: String,
    #[serde(rename = "HardwareType")]
    pub _HardwareType: String,
    /// Type of hardware
    #[serde(skip)]
    pub HardwareType: T,
    /// Identifier of parent hardware, empty string if none
    pub Parent: String,
}

/// 统一的硬件监控接口
pub trait HardwareMonitor: Sized {
    const CON_QUERY: &'static str;
    const HW_QUERY: &'static str;
    const SENSOR_QUERY: &'static str;
    type HWType;
    type SensorType;
    fn new() -> AnyResult<Self>;
    fn test(timeout: u64) -> AnyResult<()>;
    fn stop() -> AnyResult<()>;
    fn clean() -> AnyResult<()>;
}
