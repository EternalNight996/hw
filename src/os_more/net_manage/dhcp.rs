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

/// 设置静态IP
pub async fn set_static_ip(adapter_name: &str, ip: &str, netmask: &str, gateway: Option<&str>) -> e_utils::AnyResult<String> {
  let res = if cfg!(target_os = "windows") {
    let mut args = vec![
      "interface".to_string(),
      "ip".to_string(),
      "set".to_string(),
      "address".to_string(),
      adapter_name.to_string(),
      "static".to_string(),
      ip.to_string(),
      netmask.to_string(),
    ];

    if let Some(gw) = gateway {
      args.push(gw.to_string());
    }

    let output = Cmd::new("netsh").args(args).a_output().await?;
    if output.status.success() {
      output.stdout
    } else {
      return Err(output.stdout.into());
    }
  } else if cfg!(target_os = "macos") {
    let mut cmds = vec![
      Cmd::new("sudo")
        .args(["ifconfig", adapter_name, ip, "netmask", netmask])
        .a_output()
        .await?,
    ];

    if let Some(gw) = gateway {
      cmds.push(Cmd::new("sudo").args(["route", "add", "default", gw]).a_output().await?);
    }

    if let Some(cmd) = cmds.last() {
      if cmd.status.success() {
        cmd.stdout.clone()
      } else {
        return Err(cmd.stdout.clone().into());
      }
    } else {
      return Err("Failed to set static IP".to_string().into());
    }
  } else {
    // Linux
    let mut cmds = vec![
      // 删除旧的IP配置
      Cmd::new("sudo").args(["ip", "addr", "flush", "dev", adapter_name]).a_output().await?,
      // 设置新的IP和子网掩码
      Cmd::new("sudo")
        .args(["ip", "addr", "add", &format!("{}/{}", ip, netmask), "dev", adapter_name])
        .a_output()
        .await?,
    ];

    if let Some(gw) = gateway {
      // 删除默认路由
      let _ = Cmd::new("sudo").args(["ip", "route", "del", "default"]).a_output().await;
      // 添加新的默认路由
      cmds.push(Cmd::new("sudo").args(["ip", "route", "add", "default", "via", gw]).a_output().await?);
    }

    if let Some(cmd) = cmds.last() {
      if cmd.status.success() {
        cmd.stdout.clone()
      } else {
        return Err(cmd.stdout.clone().into());
      }
    } else {
      return Err("Failed to set static IP".to_string().into());
    }
  };
  Ok(format!(
    "成功设置静态IP:{} {} {} {} {}",
    adapter_name,
    ip,
    netmask,
    gateway.unwrap_or(""),
    res
  ))
}

/// 设置静态DNS
pub async fn set_static_dns(adapter_name: &str, primary_dns: &str, secondary_dns: Option<&str>) -> e_utils::AnyResult<String> {
  let res = if cfg!(target_os = "windows") {
    let mut args = vec![
      "interface".to_string(),
      "ip".to_string(),
      "set".to_string(),
      "dns".to_string(),
      adapter_name.to_string(),
      "static".to_string(),
      primary_dns.to_string(),
    ];

    let res = Cmd::new("netsh").args(&args).a_output().await?;
    let mut s = if res.status.success() {
      res.stdout
    } else {
      return Err(res.stdout.into());
    };
    if let Some(secondary) = secondary_dns {
      args = vec![
        "interface".to_string(),
        "ip".to_string(),
        "add".to_string(),
        "dns".to_string(),
        adapter_name.to_string(),
        secondary.to_string(),
      ];
      let res = Cmd::new("netsh").args(args).a_output().await?;
      if res.status.success() {
        s.push_str(&res.stdout);
      } else {
        return Err(res.stdout.into());
      }
    }
    s
  } else if cfg!(target_os = "macos") {
    let dns_servers = match secondary_dns {
      Some(secondary) => format!("{} {}", primary_dns, secondary),
      None => primary_dns.to_string(),
    };

    let res = Cmd::new("sudo")
      .args(["networksetup", "-setdnsservers", adapter_name, &dns_servers])
      .a_output()
      .await?;
    if res.status.success() {
      res.stdout
    } else {
      return Err(res.stdout.into());
    }
  } else {
    // Linux
    let resolv_conf = match secondary_dns {
      Some(secondary) => format!("nameserver {}\nnameserver {}\n", primary_dns, secondary),
      None => format!("nameserver {}\n", primary_dns),
    };

    // 写入临时文件
    let temp_file = "/tmp/resolv.conf.temp";
    std::fs::write(temp_file, resolv_conf)?;

    // 移动到正确位置
    let res = Cmd::new("sudo").args(["mv", temp_file, "/etc/resolv.conf"]).a_output().await?;
    if res.status.success() {
      res.stdout
    } else {
      return Err(res.stdout.into());
    }
  };
  Ok(format!(
    "成功设置静态DNS:{} {} {} {}",
    adapter_name,
    primary_dns,
    secondary_dns.unwrap_or(""),
    res
  ))
}

/// 验证IP配置是否生效
pub async fn verify_ip_config(adapter_name: &str, expected_ip: &str) -> e_utils::AnyResult<bool> {
  let output = if cfg!(target_os = "windows") {
    Cmd::new("ipconfig").args(["/all"]).a_output().await?
  } else {
    Cmd::new("ip").args(["addr", "show", adapter_name]).a_output().await?
  };
  if output.status.success() {
    Ok(output.stdout.contains(expected_ip))
  } else {
    return Err(output.stdout.into());
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_verify_ip_config() {
    // 这里使用一个本地回环地址作为测试
    let loopback = if cfg!(windows) { "127.0.0.1" } else { "lo" };

    let result = verify_ip_config(loopback, "127.0.0.1").await;
    assert!(result.is_ok());
    assert!(result.unwrap());
  }
}
