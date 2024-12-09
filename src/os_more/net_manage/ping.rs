use e_utils::cmd::Cmd;

/// 检查目标是否可以 ping 通
pub async fn ping(source: &str, target: &str, count: &str) -> e_utils::AnyResult<String> {
  let args = if cfg!(target_os = "windows") {
    ["ping", "-i", source, "-n", count, "-w", "1000", target] // 修改 Windows 的源 IP 参数
  } else {
    ["ping", "-c", count, "-W", "1", "-S", source, target]
  };
  let res = Cmd::new(&args[0]).args(&args[1..]).a_output().await?;
  if res.status.success() && res.stdout.contains("0%") {
    Ok(res.stdout)
  } else {
    Err(res.stdout.into())
  }
}
