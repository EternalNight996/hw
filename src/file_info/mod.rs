#[cfg(target_os = "windows")]
mod win;
#[cfg(target_os = "windows")]
pub use win::*;
#[cfg(target_os = "linux")]
mod unix;
#[cfg(target_os = "linux")]
pub use unix::*;

pub mod api;
pub use api::*;
pub mod ty;
pub use ty::*;
