#[cfg(target_os = "windows")]
#[cfg(feature = "aida64")]
pub mod aida64;
#[cfg(target_os = "windows")]
#[cfg(feature = "ohm")]
pub mod ohm;
#[cfg(feature = "sysinfo")]
pub mod os;
pub mod wmic;

#[cfg(feature = "cli")]
pub mod cli;
#[cfg(feature = "cli")]
pub use cli::*;

pub mod common;
pub mod api_test;