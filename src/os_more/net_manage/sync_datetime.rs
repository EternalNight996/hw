use std::time::Duration;

use e_utils::cmd::Cmd;

#[cfg(windows)]
async fn ensure_windows_time_service() -> e_utils::AnyResult<()> {
  let status = Cmd::new("w32tm").args(["/query", "/status"]).a_output().await?;
  crate::p(&status.stdout);
  if !status.stdout.contains("Leap") {
    crate::p("正在重新配置 Windows 时间服务...");

    // 重新注册服务
    let _ = Cmd::new("w32tm")
      .args(["/unregister"])
      .a_output()
      .await
      .inspect(|v| crate::p(format!("取消注册服务: {}", v.stdout)));

    let _ = Cmd::new("w32tm")
      .args(["/register"])
      .a_output()
      .await
      .inspect(|v| crate::p(format!("注册服务: {}", v.stdout)));

    // 启动服务
    let start_result = Cmd::new("net").args(["start", "w32time"]).a_output().await?;

    crate::p(format!("启动服务: {}", start_result.stdout));

    if !start_result.status.success() {
      return Err("无法启动 Windows 时间服务".into());
    }

    tokio::time::sleep(Duration::from_secs(2)).await;
  }

  Ok(())
}

pub async fn sync_datetime(server: &str) -> e_utils::AnyResult<String> {
  #[cfg(not(windows))]
  return Err("不支持的系统".into());
  #[cfg(windows)]
  {
    ensure_windows_time_service().await?;

    let config_result = Cmd::new("w32tm")
      .args([
        "/config",
        &format!("/manualpeerlist:{}", server),
        "/syncfromflags:manual",
        "/reliable:yes",
        "/update",
      ])
      .a_output()
      .await?;

    if !config_result.status.success() {
      return Err("配置时间服务器失败".into());
    }

    // 尝试同步时间，最多重试3次
    for i in 1..=3 {
      crate::wp(format!("正在进行第 {i} 次时间同步尝试连接 {server}..."));

      let sync_result = Cmd::new("w32tm").args(["/resync", "/force"]).a_output().await?;
      if sync_result.status.success()
        && (sync_result.stdout.contains("成功")
          || sync_result.stdout.contains("success")
          || sync_result.stdout.contains("已成功完成")
          || sync_result.stdout.is_empty())
      {
        return Ok(format!("时间同步{server}成功",));
      }

      if i < 3 {
        crate::wp("同步失败，等待重试...");
        tokio::time::sleep(Duration::from_secs(1)).await;
      }
    }

    Err("时间同步失败，请检查网络连接".into())
  }
}
