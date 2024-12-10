#[cfg(all(target_os = "windows", feature = "file-info"))]
mod win;
#[cfg(all(target_os = "windows", feature = "file-info"))]
pub use win::*;
#[cfg(all(target_os = "linux", feature = "file-info"))]
mod unix;
#[cfg(all(target_os = "linux", feature = "file-info"))]
pub use unix::*;
pub mod api;
pub use api::*;
#[cfg(feature = "file-info")]
pub mod ty;
#[cfg(feature = "file-info")]
pub use ty::*;
