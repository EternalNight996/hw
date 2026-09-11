#[tokio::main]
async fn main() -> e_utils::AnyResult<()> {
  // --res 结果模式：明细只写 logs/hw.log，标准输出只留 R<...>R 协议行
  let res = std::env::args().any(|a| a == "--res");
  hw::log::init_logging_with(res);
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
    // 结果：--res 才把 R<...>R 写标准输出（etest-core 从 stdout 解析）；
    // 无论是否 --res，都追加为结果日志 logs/hw.log 的最后一行
    let rr = hw::rr_line(&content, status);
    if res {
      println!("{rr}");
    }
    hw::write_result_line(&rr);
    return Ok(());
  }
  #[cfg(not(feature = "cli"))]
  Err("请开启特性 cli".into())
}
