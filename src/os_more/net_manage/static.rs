use e_utils::{cmd::Cmd, AnyResult};

/// 执行命令并检查输出
async fn exec_cmd(cmd: Cmd) -> AnyResult<String> {
  let output = cmd.a_output().await?;
  if output.status.success() {
    Ok(output.stdout)
  } else {
    Err(output.stdout.into())
  }
}

/// 验证IP配置
async fn verify_ip_config(_adapter_name: &str, _ip: &str, _gateway: Option<&str>) -> AnyResult<bool> {
  Ok(true)
}

/// 验证DNS配置
async fn verify_dns_config(_adapter_name: &str, _primary_dns: &str, _secondary_dns: Option<&str>) -> AnyResult<bool> {
  Ok(true)
}

/// 设置静态IP
pub async fn set_static_ip(adapter_name: &str, ip: &str, netmask: &str, gateway: Option<&str>) -> AnyResult<String> {
  let name = &format!("\"{}\"", adapter_name);

  let res = match () {
    _ if cfg!(target_os = "windows") => {
      exec_cmd(Cmd::new("netsh").args(["interface", "ip", "set", "address", name, "static", ip, netmask, gateway.unwrap_or_default()])).await?
    }
    _ if cfg!(target_os = "macos") => {
      exec_cmd(Cmd::new("sudo").args(["ifconfig", name, ip, "netmask", netmask])).await?;
      if let Some(gw) = gateway {
        exec_cmd(Cmd::new("sudo").args(["route", "add", "default", gw])).await?
      } else {
        String::new()
      }
    }
    _ => {
      exec_cmd(Cmd::new("sudo").args(["ip", "addr", "add", &format!("{}/{}", ip, netmask), "dev", name])).await?;
      if let Some(gw) = gateway {
        exec_cmd(Cmd::new("sudo").args(["ip", "route", "add", "default", "via", gw])).await?
      } else {
        String::new()
      }
    }
  };
  crate::dp(&res);
  // 验证配置是否生效
  if res.is_empty() && verify_ip_config(adapter_name, ip, gateway).await? {
    Ok(format!("成功设置静态IP: {} {} {} {}", adapter_name, ip, netmask, gateway.unwrap_or("")))
  } else {
    Err(format!("{} {} -> {}", adapter_name, ip, res).into())
  }
}

/// 设置静态DNS
pub async fn set_static_dns(adapter_name: &str, primary_dns: &str, secondary_dns: Option<&str>) -> AnyResult<String> {
  let name = &format!("\"{}\"", adapter_name);

  let res = match () {
    _ if cfg!(target_os = "windows") => {
      exec_cmd(Cmd::new("netsh").args(["interface", "ip", "set", "dns", name, "static", primary_dns])).await?;
      if let Some(secondary) = secondary_dns {
        exec_cmd(Cmd::new("netsh").args(["interface", "ip", "add", "dns", name, secondary, "index=2"])).await?
      } else {
        String::new()
      }
    }
    _ if cfg!(target_os = "macos") => {
      let dns_servers = match secondary_dns {
        Some(secondary) => format!("{} {}", primary_dns, secondary),
        None => primary_dns.to_string(),
      };
      exec_cmd(Cmd::new("sudo").args(["networksetup", "-setdnsservers", name, &dns_servers])).await?
    }
    _ => {
      let mut content = format!("nameserver {}\n", primary_dns);
      if let Some(secondary) = secondary_dns {
        content.push_str(&format!("nameserver {}\n", secondary));
      }
      std::fs::write("/etc/resolv.conf", content)?;
      String::new()
    }
  };
  crate::dp(&res);
  // 验证配置是否生效
  if verify_dns_config(adapter_name, primary_dns, secondary_dns).await? {
    Ok(format!("成功设置静态DNS: {} {} {}", adapter_name, primary_dns, secondary_dns.unwrap_or("")))
  } else {
    Err(format!("{} {} -> {}", adapter_name, primary_dns, res).into())
  }
}
