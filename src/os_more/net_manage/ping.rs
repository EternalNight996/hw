use e_utils::cmd::Cmd;

/// 检查目标是否可以 ping 通
pub async fn ping(source: &str, target: &str, count: &str, fail_count: usize) -> e_utils::AnyResult<String> {
  let mut err = String::new();
  for i in 0..fail_count {
    let args = if cfg!(target_os = "windows") {
      ["ping", "-i", source, "-n", count, "-w", "1000", target] // 修改 Windows 的源 IP 参数
    } else {
      ["ping", "-c", count, "-W", "1", "-S", source, target]
    };
    let res = Cmd::new(&args[0]).args(&args[1..]).a_output().await?;
    if res.status.success() && res.stdout.contains("0%") {
      return Ok(res.stdout);
    }
    err = res.stdout;
    crate::wp(format!("重试尝试网络连接 {} 次", i + 1));
  }
  crate::ep(format!("{}", err));
  return Err(err.into());
}
