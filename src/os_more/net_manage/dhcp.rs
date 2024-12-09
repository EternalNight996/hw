use e_utils::cmd::Cmd;

/// 设置IP DHCP
pub async fn set_ip_dhcp(adapter_name: &str) -> e_utils::AnyResult<String> {
  if cfg!(target_os = "windows") {
    Ok(
      Cmd::new("netsh")
        .args(["interface", "ip", "set", "address", adapter_name, "dhcp"])
        .a_output()
        .await?
        .stdout,
    )
  } else if cfg!(target_os = "macos") {
    Ok(Cmd::new("sudo").args(["ipconfig", "set", adapter_name, "DHCP"]).a_output().await?.stdout)
  } else {
    // Linux: 先停止之前的 dhclient，然后重新启动
    let _ = Cmd::new("sudo").args(["pkill", "dhclient"]).a_output().await;

    Ok(Cmd::new("sudo").args(["dhclient", adapter_name]).a_output().await?.stdout)
  }
}

/// 设置DNS DHCP
pub async fn set_dns_dhcp(adapter_name: &str) -> e_utils::AnyResult<String> {
  if cfg!(target_os = "windows") {
    Ok(
      Cmd::new("netsh")
        .args(["interface", "ip", "set", "dns", adapter_name, "dhcp"])
        .a_output()
        .await?
        .stdout,
    )
  } else if cfg!(target_os = "macos") {
    Ok(
      Cmd::new("sudo")
        .args(["networksetup", "-setdnsservers", adapter_name, "empty"])
        .a_output()
        .await?
        .stdout,
    )
  } else {
    // Linux: 使用 resolvconf 管理 DNS
    let res = Cmd::new("sudo").args(["resolvconf", "-d", adapter_name]).a_output().await;

    match res {
      Ok(_) => Ok("DNS set to DHCP successfully".to_string()),
      Err(_) => {
        // 如果 resolvconf 不可用，回退到直接修改 resolv.conf
        let _ = Cmd::new("sudo").args(["rm", "-f", "/etc/resolv.conf"]).a_output().await?;
        Ok(
          Cmd::new("sudo")
            .args(["ln", "-s", "/run/systemd/resolve/resolv.conf", "/etc/resolv.conf"])
            .a_output()
            .await?
            .stdout,
        )
      }
    }
  }
}
