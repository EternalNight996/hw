
/// 将字节转换为 GiB，保留两位小数
pub fn bytes_to_gib(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0 * 1024.0)
  }