#[cfg(all(feature = "aida64", target_os = "windows"))]
pub mod aida64;
pub mod drive;
pub mod file_info;
#[cfg(all(feature = "ohm", target_os = "windows"))]
pub mod ohm;
#[cfg(feature = "os")]
pub mod os;
#[cfg(feature = "os-office")]
pub mod os_office;
#[cfg(feature = "os-system")]
pub mod os_system;

#[cfg(feature = "cli")]
pub mod cli;
#[cfg(feature = "cli")]
pub use cli::*;

#[cfg(feature = "disk")]
pub mod disk;

pub mod api_test;
pub mod common;
pub mod os_more;
pub mod share;
pub mod wmic;
pub use share::{dp, ep, p, wp};
