#[cfg(feature = "cli")]
#[tokio::main]
async fn main() -> e_utils::AnyResult<()> {
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
    Err(e) => res.content = e.to_string(),
  }
  println!("\n{}", res.to_str()?);
  Ok(())
}

#[cfg(not(feature = "cli"))]
fn main() {
  println!("请开启特性 cli");
}
