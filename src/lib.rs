#[cfg(all(feature = "aida64", target_os = "windows"))]
pub mod aida64;
#[cfg(all(feature = "ohm", target_os = "windows"))]
pub mod ohm;
pub mod os;
pub mod wmic;

#[cfg(feature = "cli")]
pub mod cli;
#[cfg(feature = "cli")]
pub use cli::*;

pub mod common;
pub mod api_test;