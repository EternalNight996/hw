//! # Hardware Monitor
//!
//! [中文文档](https://github.com/e-app/hw/blob/main/readme.zh.md)
//!
//! [English Document](https://github.com/e-app/hw/blob/main/readme.md)

#![doc = include_str!("../readme.md")]
#![allow(
  clippy::cognitive_complexity,
  clippy::large_enum_variant,
  clippy::module_inception,
  clippy::needless_doctest_main
)]
#![deny(unused_must_use)]
#![doc(test(no_crate_inject, attr(deny(warnings, rust_2018_idioms), allow(dead_code, unused_variables))))]
// #![warn(missing_debug_implementations, missing_docs, rust_2018_idioms, unreachable_pub)]

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
#[cfg(all(feature = "core-temp", target_os = "windows"))]
pub mod core_temp;
pub mod api_test;
pub mod common;
pub mod os_more;
pub mod share;
pub mod wmic;
pub use share::{dp, ep, p, wp};
