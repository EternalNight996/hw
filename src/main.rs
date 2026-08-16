#[tokio::main]
async fn main() -> e_utils::AnyResult<()> {
  hw::log::init_logging();
  #[cfg(feature = "cli")]
  {
    use hw::cli::api;
    use hw::cli::Opts;
    let opts = Opts::new(None as Option<Vec<&str>>)?;
    let content: String;
    let mut status = false;
    match api(opts, &mut serde_json::Value::Null).await {
      Ok(v) => {
        content = v;
        status = true;
      }
      Err(e) => {
        content = e.to_string();
      }
    }
    // 明细：结果内容以 e-log 方式输出（时间戳+级别，无 R<...>R 包装）
    if status {
      hw::p(&content);
    } else {
      hw::ep(&content);
    }
    // 结果：R<...>R 作为结果日志的最后一行追加（etest 读取）
    hw::write_result_line(&hw::rr_line(&content, status));
    return Ok(());
  }
  #[cfg(not(feature = "cli"))]
  Err("请开启特性 cli".into())
}
