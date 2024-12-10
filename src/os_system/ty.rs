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
