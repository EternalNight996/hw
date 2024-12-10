/// 将字节转换为 GiB，保留两位小数
pub fn bytes_to_gib(bytes: u64) -> f64 {
  bytes as f64 / (1024.0 * 1024.0 * 1024.0)
}

pub fn p(v: impl AsRef<str>) {
  #[cfg(feature = "cli")]
  println!("{}", v.as_ref());
  #[cfg(any(feature = "log", feature = "tracing"))]
  e_log::info!("{}", v.as_ref());
}
pub fn ep(v: impl AsRef<str>) {
  #[cfg(feature = "cli")]
  println!("{}", v.as_ref());
  #[cfg(any(feature = "log", feature = "tracing"))]
  e_log::error!("{}", v.as_ref());
}
pub fn wp(v: impl AsRef<str>) {
  #[cfg(feature = "cli")]
  println!("{}", v.as_ref());
  #[cfg(any(feature = "log", feature = "tracing"))]
  e_log::warn!("{}", v.as_ref());
}
pub fn dp(v: impl AsRef<str>) {
  #[cfg(feature = "cli")]
  println!("{}", v.as_ref());
  #[cfg(any(feature = "log", feature = "tracing"))]
  e_log::debug!("{}", v.as_ref());
}
