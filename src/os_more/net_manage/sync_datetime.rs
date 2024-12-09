use e_utils::cmd::Cmd;

pub async fn sync_datetime(arg: &str) -> e_utils::AnyResult<String> {
  if cfg!(target_os = "windows") {
    // Windows 时间同步逻辑保持不变
    let res = Cmd::new("w32tm").args(["/query", "/status"]).a_output().await?;
    if res.status.success() && res.stdout.contains("0x") {
      let _ = Cmd::new("net").args(["start", "w32time"]).a_output().await?;
    }
    let _ = Cmd::new("w32tm")
      .args(["/config", &format!("/manualpeerlist:{arg}"), "/syncfromflags:manual", "/update"])
      .a_output()
      .await?;
    let res = Cmd::new("w32tm").args(["/resync"]).a_output().await?.stdout;
    if res.contains("成功") || res.contains("success") {
      Ok(res)
    } else {
      Err(res.into())
    }
  } else if cfg!(target_os = "macos") {
    // macOS 时间同步
    let res = Cmd::new("sudo").args(["sntp", "-sS", arg]).a_output().await?.stdout;
    Ok(res)
  } else {
    // Linux 时间同步，先尝试 chronyd，失败则使用 ntpdate
    let chrony_result = Cmd::new("sudo")
      .args(["chronyd", "-q", &format!("server {}", arg)])
      .a_output()
      .await;

    match chrony_result {
      Ok(output) => Ok(output.stdout),
      Err(_) => {
        // 如果 chronyd 失败，尝试 ntpdate
        let _ = Cmd::new("sudo").args(["systemctl", "stop", "systemd-timesyncd"]).a_output().await?;

        let res = Cmd::new("sudo").args(["ntpdate", arg]).a_output().await?.stdout;
        Ok(res)
      }
    }
  }
}
