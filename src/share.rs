#![allow(unused)]
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

use std::path::Path;

use e_utils::{fs::AutoPath as _, MyParseFormat as _};
use tokio::io::AsyncReadExt as _;

/// 激活存储本地类型
#[derive(Debug, Clone)]
pub enum ActiveLocalType {
  Temp(String),
}

impl ActiveLocalType {
  /// # 清除激活码持久化
  pub fn clean_cache(self) -> e_utils::AnyResult<String> {
    match self {
      ActiveLocalType::Temp(fname) => {
        let tmp = "%TEMP%".parse_env()?;
        let path = Path::new(&tmp).join(&format!("os-key-{fname}"));
        if (path.exists() && path.auto_remove_file().is_ok()) || !path.exists() {
          Ok(format!("清除本地激活码"))
        } else {
          Err(format!("Error: Clean Cache;{}", path.display()).into())
        }
      }
    }
  }
  /// # 查询激活码持久化
  pub async fn query_cache(self) -> e_utils::AnyResult<String> {
    match self {
      ActiveLocalType::Temp(fname) => {
        let tmp = "%TEMP%".parse_env()?;
        let path = Path::new(&tmp).join(&format!("os-key-{fname}"));
        if path.exists() {
          let mut f = tokio::fs::OpenOptions::new().read(true).open(&path).await?;
          let mut sbuf = String::new();
          if f.read_to_string(&mut sbuf).await.is_ok() && sbuf.len() > 3 {
            return Ok(sbuf);
          }
        }
        Err(format!("Error: Query Cache;{}", path.display()).into())
      }
    }
  }
}
