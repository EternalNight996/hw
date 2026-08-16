#![allow(unused)]
/// 将字节转换为 GiB，保留两位小数
pub fn bytes_to_gib(bytes: u64) -> f64 {
  bytes as f64 / (1024.0 * 1024.0 * 1024.0)
}

// 日志明细：全部走 e-log（文件 + stderr），不输出到 stdout、不含 R<...>R 包装。
// 产测结束时，R<...>R 结果作为“结果日志”的最后一行追加（见 write_result_line）。
pub fn p(v: impl AsRef<str>) {
  #[cfg(any(feature = "log", feature = "tracing"))]
  e_log::info!("{}", v.as_ref());
}
pub fn ep(v: impl AsRef<str>) {
  #[cfg(any(feature = "log", feature = "tracing"))]
  e_log::error!("{}", v.as_ref());
}
pub fn wp(v: impl AsRef<str>) {
  #[cfg(any(feature = "log", feature = "tracing"))]
  e_log::warn!("{}", v.as_ref());
}
pub fn dp(v: impl AsRef<str>) {
  #[cfg(any(feature = "log", feature = "tracing"))]
  e_log::debug!("{}", v.as_ref());
}

/// 构造 R<...>R 结果行（CmdResult 结构，与 etest 约定一致）
pub fn rr_line(content: &str, status: bool) -> String {
  use e_utils::cmd::CmdResult;
  let res: CmdResult<serde_json::Value> = CmdResult {
    content: content.to_string(),
    status,
    opts: serde_json::Value::Null,
  };
  res.to_str().unwrap_or_default()
}

/// 将 R<...>R 结果追加为结果日志（config log_file，默认 hw-gui-test.log）的最后一行
pub fn write_result_line(rr: &str) {
  let path = crate::gui_config::HwConfig::result_log_path();
  if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
    use std::io::Write;
    let _ = writeln!(f, "{}", rr);
  }
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
