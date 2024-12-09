use serde::{Deserialize, Serialize};
use strum::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display, EnumString, Default, VariantArray)]
pub enum Type {
  #[default]
  ALL,
  CpuName,
  MemoryTotal,
  CpuCoreCount,
  OsVersion,
  OsFullVersion,
  KernelVersion,
  HostName,
  Uptime,
  CpuUsage,
  MemoryUsage,
  CpuArch,
  UserNames,
  NetInterface,
  NetManage,
  Desktop,
  Drive,
}

pub async fn query_os_more<T: AsRef<str>>(
  infos: &[Type],
  args: impl IntoIterator<Item = T>,
  filter: impl IntoIterator<Item = T>,
  is_full: bool,
) -> e_utils::AnyResult<Vec<String>> {
  let mut results = vec![];
  let args = args.into_iter().map(|v| v.as_ref().to_string()).collect::<Vec<_>>();
  let filter = filter.into_iter().map(|v| v.as_ref().to_string()).collect::<Vec<_>>();
  for info in infos {
    let res = system_query(info)?;
    if !res.is_empty() {
      results.push(format!("{}={}", info, res));
    }
    let res = user_query(info, &args, &filter)?;
    if !res.is_empty() {
      results.push(format!("{}={}", info, res));
    }
    let res = network_query(info, &args, &filter, is_full).await?;
    if !res.is_empty() {
      results.push(format!("{}={}", info, res));
    }
  }
  if results.is_empty() {
    return Err("没有查询到任何信息".into());
  }
  Ok(results)
}

pub mod network {
  pub const MAC_CHECKS: [&str; 4] = ["00-00-00-00-00-00", "88-88-88-88-87-88", "88-88-88-88-88-88", "TO BE FILLED BY O.E.M."];
}
/// 网络查询
pub async fn network_query(info: &super::Type, args: &Vec<String>, filter: &Vec<String>, is_full: bool) -> e_utils::AnyResult<String> {
  #[cfg(not(feature = "network"))]
  return Ok(String::new());
  #[cfg(feature = "network")]
  {
    let task = args.get(0).map(|x| x.as_str()).unwrap_or_default();
    match info {
      super::Type::NetInterface => {
        return match task {
          "old" => Ok(serde_json::to_string_pretty(
            &sysinfo::Networks::new_with_refreshed_list()
              .iter()
              .map(|(k, _)| k.clone())
              .collect::<Vec<_>>(),
          )?),
          "print" => {
            if is_full {
              let ifaces = crate::os_more::net_interface::get_interfaces();
              let count = ifaces.len();
              for iface in ifaces {
                println!("{}", serde_json::to_string_pretty(&iface)?)
              }
              Ok(format!("Count: {}", count))
            } else {
              let ifaces = crate::os_more::net_interface::get_interfaces_simple(filter.clone());
              let count = ifaces.len();
              for iface in ifaces {
                println!("{}", serde_json::to_string_pretty(&iface)?)
              }
              Ok(format!("Count: {}", count))
            }
          }
          "check-mac" => {
            let ifaces = crate::os_more::net_interface::get_interfaces_simple(filter.clone());
            // Check each interface's MAC address
            for iface in &ifaces {
              let ref mac = iface.mac_addr;
              // Check against invalid MAC patterns
              if network::MAC_CHECKS.contains(&mac.as_str()) {
                return Err(format!("FAIL:{} ->  {} 未烧录MAC地址", iface.friendly_name, mac).into());
              }
              // Check for duplicate MACs
              let find_repect = ifaces.iter().find(|i| &i.mac_addr == mac && i.friendly_name != iface.friendly_name);

              if let Some(repeat_mac) = find_repect {
                return Err(format!("FAIL: {} 重复MAC地址: {}", repeat_mac.friendly_name, mac).into());
              }
            }
            Ok(serde_json::to_string_pretty(&ifaces)?)
          }
          "nodes" => {
            if is_full {
              Ok(serde_json::to_string_pretty(&crate::os_more::net_interface::get_interfaces())?)
            } else {
              Ok(serde_json::to_string_pretty(&crate::os_more::net_interface::get_interfaces_simple(
                filter.clone(),
              ))?)
            }
          }
          _ => Ok(String::new()),
        };
      }
      super::Type::NetManage => {
        return match task {
          "auto-dhcp" => {
            let mut new = vec![];
            for iface in crate::os_more::net_interface::get_interfaces_simple(filter.clone()) {
              let ip_res = crate::os_more::net_manage::set_ip_dhcp(&iface.friendly_name).await?;
              let dns_res = crate::os_more::net_manage::set_dns_dhcp(&iface.friendly_name).await?;
              new.push(serde_json::json!({
                "name": iface.friendly_name,
                "type": iface.if_type,
                "dnsRes": dns_res,
                "ipRes": ip_res
              }));
            }
            Ok(serde_json::to_string_pretty(&new)?)
          }
          "sync-datetime" => {
            let arg = args.get(1).cloned().unwrap_or("time.windows.com".to_string());
            Ok(crate::os_more::net_manage::sync_datetime(&arg).await?)
          }
          "ping" => {
            let source = args.get(1).cloned().ok_or("Args Error Source 0 ")?;
            let target = args.get(2).cloned().ok_or("Args Error Target 1 ")?;
            let count = args.get(3).cloned().ok_or("Args Error Count 2 ")?;
            Ok(crate::os_more::net_manage::ping(&source, &target, &count).await?)
          }
          "interfaces-ping" => {
            let target = args.get(1).cloned().ok_or("Args Error Target 1 ")?;
            let count = args.get(2).cloned().ok_or("Args Error Count 2 ")?;
            #[cfg(target_os = "windows")]
            let faces = crate::os_more::net_interface::get_interfaces_simple(filter.clone());
            #[cfg(not(target_os = "windows"))]
            let faces = vec![];
            if faces.is_empty() {
              return Err("No interfaces found".into());
            }
            // 创建一个异步任务列表
            let handles: Vec<_> = faces
              .iter()
              .map(|face| {
                let target = target.clone();
                let count = count.clone();
                async move { crate::os_more::net_manage::ping(&face.ipv4, &target, &count).await }
              })
              .collect();
            // 并发执行所有任务并处理结果
            let results = futures::future::try_join_all(handles).await?;
            // 将所有响应合并为一个字符串
            let response = results.join("\n");
            Ok(response)
          }
          _ => Ok(String::new()),
        };
      }
      _ => Ok(String::new()),
    }
  }
}
pub fn user_query(info: &Type, args: &Vec<String>, filter: &Vec<String>) -> e_utils::AnyResult<String> {
  #[cfg(not(feature = "user"))]
  return Ok(String::new());
  #[cfg(feature = "user")]
  {
    match info {
      super::Type::UserNames => {
        let users = sysinfo::Users::new_with_refreshed_list()
          .list()
          .iter()
          .map(|user| user.id().to_string())
          .collect::<Vec<String>>()
          .join(",");
        Ok(users)
      }
      super::Type::Desktop => {
        let task = args.get(0).map(|x| x.as_str()).unwrap_or_default();
        let attr_filter = args.get(1).and_then(|v| v.parse::<u32>().ok()).filter(|&v| v > 0);
        let query_user = args.get(2).cloned();
        return match task {
          "print" => {
            let mut items = crate::os_more::user_desktop::get_desktop_items(query_user, attr_filter, filter);
            items.dedup_by_key(|v| v.path.clone());
            let count = items.len();
            for item in &items {
              let is_dir = if item.is_dir { "目录" } else { "" };
              let is_hidden = if item.is_hidden { "隐藏" } else { "" };
              println!(
                "[{}] 用户[{}] 属性[{}] {} {}",
                item.path.display(),
                item.uname,
                item.attribute,
                is_dir,
                is_hidden
              );
            }
            Ok(format!("Count: {count}"))
          }
          "nodes" => {
            let mut items = crate::os_more::user_desktop::get_desktop_items(query_user, attr_filter, filter);
            items.dedup_by_key(|v| v.path.clone());
            Ok(serde_json::to_string_pretty(&items)?)
          }
          _ => Ok(String::new()),
        };
      }
      _ => Ok(String::new()),
    }
  }
}
pub fn system_query(info: &super::Type) -> e_utils::Result<String> {
  #[cfg(feature = "system")]
  {
    match info {
      super::Type::CpuName => system::cpu_name(),
      super::Type::MemoryTotal => system::memory_total().map(|v| format!("{}GB", v)),
      super::Type::CpuCoreCount => system::cpu_core_count().map(|v| v.to_string()),
      super::Type::OsVersion => system::os_version(),
      super::Type::OsFullVersion => Ok(format!("{} {}", system::os_full_version()?, system::kernel_version()?)),
      super::Type::KernelVersion => system::kernel_version(),
      super::Type::HostName => system::host_name(),
      super::Type::Uptime => system::uptime().map(|v| format!("{}秒", v)),
      super::Type::CpuUsage => system::cpu_usage().map(|v| format!("{:.2}%", v)),
      super::Type::MemoryUsage => system::memory_usage().map(|v| format!("{:.2}%", v)),
      super::Type::CpuArch => system::cpu_arch(),
      _ => Ok(String::new()),
    }
  }
}
#[cfg(feature = "system")]
pub mod system {
  /// 获取系统运行时间
  pub fn uptime() -> e_utils::Result<u64> {
    Ok(sysinfo::System::uptime())
  }
  /// 获取 CPU 使用率
  pub fn cpu_usage() -> e_utils::Result<f32> {
    let mut system = sysinfo::System::new();
    system.refresh_cpu_usage();
    Ok(system.global_cpu_usage())
  }
  /// 获取内存使用率
  pub fn memory_usage() -> e_utils::Result<f32> {
    let mut system = sysinfo::System::new();
    system.refresh_memory();
    Ok(system.used_memory() as f32 / system.total_memory() as f32)
  }
  /// 获取 CPU 核心数
  pub fn cpu_core_count() -> e_utils::Result<usize> {
    let mut system = sysinfo::System::new();
    system.refresh_cpu_frequency();
    Ok(system.cpus().len())
  }
  /// 获取 CPU 产品名称
  pub fn cpu_name() -> e_utils::Result<String> {
    let mut system = sysinfo::System::new();
    system.refresh_cpu_frequency();
    system
      .cpus()
      .first()
      .map(|cpu| cpu.brand().to_string())
      .ok_or("CPU 产品名称获取失败".into())
  }
  /// 获取内存总量
  pub fn memory_total() -> e_utils::Result<f64> {
    let mut system = sysinfo::System::new();
    system.refresh_memory();
    Ok(crate::share::bytes_to_gib(system.total_memory()).round())
  }
  /// 获取主机名
  pub fn host_name() -> e_utils::Result<String> {
    sysinfo::System::host_name().ok_or("主机名获取失败".into())
  }
  /// 获取操作系统版本
  pub fn os_version() -> e_utils::Result<String> {
    sysinfo::System::os_version().ok_or("操作系统版本获取失败".into())
  }
  /// 获取内核版本
  pub fn kernel_version() -> e_utils::Result<String> {
    sysinfo::System::kernel_version().ok_or("内核版本获取失败".into())
  }
  /// 获取 CPU 架构
  pub fn cpu_arch() -> e_utils::Result<String> {
    Ok(sysinfo::System::cpu_arch())
  }
  /// 获取完整 OS 名称
  pub fn os_full_version() -> e_utils::Result<String> {
    Ok(sysinfo::System::long_os_version().unwrap_or("未知".to_string()))
  }
}
