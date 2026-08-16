#[tokio::main]
async fn main() -> e_utils::AnyResult<()> {
  hw::log::init_logging();
  #[cfg(feature = "cli")]
  {
    use e_utils::cmd::CmdResult;
    use hw::cli::api;
    use hw::cli::Opts;
    use serde_json::Value;
    let opts = Opts::new(None as Option<Vec<&str>>)?;
    let mut res: CmdResult<Value> = CmdResult {
      content: String::new(),
      status: false,
      opts: Value::Null,
    };
    match api(opts, &mut res.opts).await {
      Ok(v) => {
        res.content = v;
        res.status = true;
      }
      Err(e) => {
        res.content = e.to_string();
      }
    }
    // stdout 只输出最后一行 R<...>R（etest 协议）；明细已由 p/ep/wp/dp 写入日志
    hw::protocol_line(&res.to_str()?);
    return Ok(());
  }
  #[cfg(not(feature = "cli"))]
  Err("请开启特性 cli".into())
}
