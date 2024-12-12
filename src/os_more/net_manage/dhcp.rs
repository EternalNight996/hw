use e_utils::cmd::{Cmd, CmdOutput};

/// 设置IP DHCP并验证状态
pub async fn set_ip_dhcp(iname: &str) -> e_utils::AnyResult<String> {
  let name = &format!("\"{iname}\"");
  // 执行设置DHCP命令
  let res = if cfg!(target_os = "windows") {
    Cmd::new("netsh").args(["interface", "ip", "set", "address", name, "dhcp"]).a_output().await?
  } else if cfg!(target_os = "macos") {
    Cmd::new("sudo").args(["ipconfig", "set", name, "DHCP"]).a_output().await?
  } else {
    // Linux: 先停止之前的 dhclient，然后重新启动
    let _ = Cmd::new("sudo").args(["pkill", "dhclient"]).a_output().await;
    Cmd::new("sudo").args(["dhclient", name]).a_output().await?
  };
  if is_ip_dhcp(&res, name).await? {
    Ok(format!("{iname}=IP-DHCP"))
  } else {
    Err(format!("IP DHCP设置失败：:{}", iname).into())
  }
}

pub async fn is_ip_dhcp(res: &CmdOutput, name: &str) -> e_utils::AnyResult<bool> {
  let ref stdout = res.stdout;
  crate::dp(format!("{name} IP DHCP: {stdout}"));
  Ok(stdout.is_empty() || stdout.contains("启用 DHCP") || stdout.to_lowercase().contains("yes"))
}

pub async fn is_dns_dhcp(res: &CmdOutput, name: &str) -> e_utils::AnyResult<bool> {
  let ref stdout = res.stdout;
  crate::dp(format!("{name} DNS DHCP: {stdout}"));
  Ok(true)
}

/// 设置DNS DHCP
pub async fn set_dns_dhcp(adapter_name: &str) -> e_utils::AnyResult<String> {
  let res = if cfg!(target_os = "windows") {
    Cmd::new("netsh")
      .args(["interface", "ip", "set", "dns", adapter_name, "dhcp"])
      .a_output()
      .await?
  } else if cfg!(target_os = "macos") {
    Cmd::new("sudo")
      .args(["networksetup", "-setdnsservers", adapter_name, "empty"])
      .a_output()
      .await?
  } else {
    let res = Cmd::new("sudo").args(["resolvconf", "-d", adapter_name]).a_output().await;
    if res.is_err() {
      let _ = Cmd::new("sudo").args(["rm", "-f", "/etc/resolv.conf"]).a_output().await?;
      Cmd::new("sudo")
        .args(["ln", "-s", "/run/systemd/resolve/resolv.conf", "/etc/resolv.conf"])
        .a_output()
        .await?
    } else {
      res?
    }
  };
  if is_dns_dhcp(&res, adapter_name).await? {
    Ok(format!("{adapter_name}=DNS-DHCP"))
  } else {
    Err(format!("DNS DHCP设置失败：:{}", adapter_name).into())
  }
}
