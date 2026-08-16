//! 内置测试模式注册


#[cfg(feature = "network")]
mod net_speed;
#[cfg(feature = "system")]
mod cpu_usage;
#[cfg(feature = "system")]
mod mem_usage;
#[cfg(feature = "disk")]
mod disk_usage;
#[cfg(all(target_os = "windows", any(feature = "ohm", feature = "lhm", feature = "aida64")))]
mod wmi_backend;
#[cfg(all(target_os = "windows", any(feature = "ohm", feature = "lhm", feature = "aida64")))]
mod temp;
#[cfg(all(target_os = "windows", any(feature = "ohm", feature = "lhm", feature = "aida64")))]
mod sensor;
#[cfg(all(target_os = "windows", any(feature = "ohm", feature = "lhm", feature = "aida64")))]
mod gpu_usage;

/// 注册所有内置模式
pub fn register_all() {
  #[cfg(feature = "network")]
  crate::test_mode::register(&net_speed::NET_SPEED);
  #[cfg(feature = "system")]
  {
    crate::test_mode::register(&cpu_usage::CPU_USAGE);
    crate::test_mode::register(&mem_usage::MEM_USAGE);
  }
  #[cfg(feature = "disk")]
  crate::test_mode::register(&disk_usage::DISK_USAGE);
  #[cfg(all(target_os = "windows", any(feature = "ohm", feature = "lhm", feature = "aida64")))]
  {
    crate::test_mode::register(&temp::TEMP);
    crate::test_mode::register(&gpu_usage::GPU_USAGE);
    crate::test_mode::register(&sensor::CPU_CLOCK);
    crate::test_mode::register(&sensor::FAN_SPEED);
    crate::test_mode::register(&sensor::VOLTAGE);
    crate::test_mode::register(&sensor::POWER);
  }
}