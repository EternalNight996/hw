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
  MemoryManufacturerPartNumber,
}
#[allow(unused)]
pub async fn query_os_more<T: AsRef<str>>(infos: &[Type], args: &[T], filter: &[T], is_full: bool) -> e_utils::AnyResult<Vec<String>> {
  let mut results = vec![];
  for info in infos {
    #[cfg(feature = "system")]
    {
      let res = system_query(info)?;
      if !res.is_empty() {
        results.push(format!("{}={}", info, res));
      }
    }
    #[cfg(feature = "user")]
    {
      let res = user_query(info, args, filter)?;
      if !res.is_empty() {
        results.push(format!("{}={}", info, res));
      }
    }
    #[cfg(feature = "network")]
    {
      let res = network_query(info, args, filter, is_full).await?;
      if !res.is_empty() {
        results.push(format!("{}={}", info, res));
      }
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
#[cfg(feature = "network")]
pub async fn network_query<T: AsRef<str>>(info: &super::Type, args: &[T], filter: &[T], is_full: bool) -> e_utils::AnyResult<String> {
  use crate::os_more::net_interface::InterfaceSimple;

  let task = args.get(0).map(|x| x.as_ref()).unwrap_or_default();
  let filter_refs: Vec<&str> = filter.iter().map(AsRef::as_ref).collect();
  match info {
    super::Type::NetInterface => {
      return match task {
        "old" => Ok(serde_json::to_string(
          &sysinfo::Networks::new_with_refreshed_list().iter().map(|(k, _)| k.clone()).collect::<Vec<_>>(),
        )?),
        "print" => {
          if is_full {
            let ifaces = crate::os_more::net_interface::get_interfaces();
            let count = ifaces.len();
            for iface in ifaces {
              crate::p(serde_json::to_string_pretty(&iface)?)
            }
            Ok(format!("Count: {}", count))
          } else {
            let ifaces = crate::os_more::net_interface::get_interfaces_simple(filter_refs)?;
            let count = ifaces.len();
            for iface in ifaces {
              crate::p(serde_json::to_string_pretty(&iface)?)
            }
            Ok(format!("Count: {}", count))
          }
        }
        "check-mac" => {
          let count = args.get(1).and_then(|x| x.as_ref().parse::<usize>().ok()).unwrap_or(0);
          let ifaces = crate::os_more::net_interface::get_interfaces_simple(filter_refs)?;
          // 提前检查数量
          if count > 0 && count != ifaces.len() {
            return Err(format!("正确网口数量:{} 实际网口数量:{}", count, ifaces.len()).into());
          }

          // 使用 HashSet 加速 MAC 检查
          use std::collections::{HashMap, HashSet};

          // 预编译无效 MAC 模式到 HashSet
          let mac_checks: HashSet<&str> = network::MAC_CHECKS.into();

          // 构建 MAC 地址映射表 (MAC -> 接口列表)
          let mut mac_map: HashMap<&str, Vec<&InterfaceSimple>> = HashMap::new();
          for iface in &ifaces {
            mac_map.entry(&iface.mac_addr).or_default().push(iface);
          }

          // 单次遍历检查所有接口
          for iface in &ifaces {
            let mac = &iface.mac_addr;
            // 检查无效 MAC（O(1) 时间）
            if mac_checks.contains(mac.as_str()) {
              return Err(
                format!(
                  "FAIL MAC地址未烧录{}, INTERFACE={}, MAC={}, TYPE={}, IP={}, STATUS={}",
                  mac, iface.friendly_name, mac, iface.if_type, iface.ipv4, iface.network_status
                )
                .into(),
              );
            }
            // 检查重复 MAC（O(1) 时间）
            if let Some(entries) = mac_map.get(mac.as_str()) {
              if entries.iter().any(|e| e.friendly_name != iface.friendly_name) {
                let repeat_mac = entries.iter().find(|e| e.friendly_name != iface.friendly_name).ok_or("重复MAC地址获取失败")?;
                return Err(
                  format!(
                    "FAIL {}重复MAC地址{}, INTERFACE={}, MAC={}, TYPE={}, IP={}, STATUS={}",
                    repeat_mac.friendly_name, mac, iface.friendly_name, mac, iface.if_type, iface.ipv4, iface.network_status
                  )
                  .into(),
                );
              }
            }

            // 延迟日志输出到检查通过后
            crate::dp(format!(
              "PASS, INTERFACE={}, MAC={}, TYPE={}, IP={}, STATUS={}, SPEED={}",
              iface.friendly_name, mac, iface.if_type, iface.ipv4, iface.network_status, iface.speed_mb
            ));
          }
          Ok(serde_json::to_string(&ifaces)?)
        }
        "nodes" => {
          if is_full {
            Ok(serde_json::to_string(&crate::os_more::net_interface::get_interfaces())?)
          } else {
            Ok(serde_json::to_string(&crate::os_more::net_interface::get_interfaces_simple(filter_refs)?)?)
          }
        }
        _ => Ok(String::new()),
      };
    }
    super::Type::NetManage => {
      return match task {
        "dhcp" => {
          let mut new = vec![];
          for iface in crate::os_more::net_interface::get_interfaces_simple(filter_refs)? {
            let ip_res = crate::os_more::net_manage::set_ip_dhcp(&iface.friendly_name).await?;
            let dns_res = crate::os_more::net_manage::set_dns_dhcp(&iface.friendly_name).await?;
            new.push(serde_json::json!({
              "name": iface.friendly_name,
              "type": iface.if_type,
              "dnsRes": dns_res,
              "ipRes": ip_res
            }));
          }
          Ok(serde_json::to_string(&new)?)
        }
        "set-ip" => {
          let mut new = vec![];
          let ip = args.get(1).ok_or("Args Error IP 1 ")?.as_ref();
          let netmask = args.get(2).ok_or("Args Error Netmask 2 ")?.as_ref();
          let gateway = args.get(3).map(AsRef::as_ref);
          for iface in crate::os_more::net_interface::get_interfaces_simple(filter_refs)? {
            let res = crate::os_more::net_manage::set_static_ip(&iface.friendly_name, &ip, &netmask, gateway).await?;
            new.push(serde_json::json!({
              "name": iface.friendly_name,
              "type": iface.if_type,
              "dnsRes": "",
              "ipRes": res
            }));
          }
          Ok(serde_json::to_string(&new)?)
        }
        "set-dns" => {
          let mut new = vec![];
          let primary_dns = args.get(1).ok_or("Args Error Primary DNS 1 ")?.as_ref();
          let secondary_dns = args.get(2).map(AsRef::as_ref);
          for iface in crate::os_more::net_interface::get_interfaces_simple(filter_refs)? {
            let res = crate::os_more::net_manage::set_static_dns(&iface.friendly_name, &primary_dns, secondary_dns).await?;
            new.push(serde_json::json!({
              "name": iface.friendly_name,
              "type": iface.if_type,
              "dnsRes": res,
              "ipRes": ""
            }));
          }
          Ok(serde_json::to_string(&new)?)
        }
        "sync-datetime" => {
          let arg = args.get(1).map(AsRef::as_ref).unwrap_or("time.windows.com");
          let is_register = if args.get(2).map(AsRef::as_ref).unwrap_or("0") == "1" { true } else { false };
          Ok(crate::os_more::net_manage::sync_datetime(&arg, is_register).await?)
        }
        "ping" => {
          let source = args.get(1).ok_or("Args Error Source 1 ")?.as_ref();
          let target = args.get(2).ok_or("Args Error Target 2 ")?.as_ref();
          let count = args.get(3).ok_or("Args Error Count 3 ")?.as_ref();
          let fail_count = args
            .get(4)
            .ok_or("Args Error Repeat 4 ")
            .map(AsRef::as_ref)
            .unwrap_or_default()
            .parse::<usize>()
            .unwrap_or(3);
          let res = crate::os_more::net_manage::ping(&source, &target, &count, fail_count).await?;
          crate::p(res);
          Ok("PASS".to_string())
        }
        "ping-nodes" => {
          let target = args.get(1).ok_or("Args Error Target 1 ")?.as_ref();
          let secs = args.get(2).ok_or("Args Error run time secs 2 ")?.as_ref();
          let count: usize = args.get(3).and_then(|v| v.as_ref().parse::<usize>().ok()).unwrap_or(0);
          let fail_count = args
            .get(4)
            .ok_or("Args Error Repeat 4 ")
            .map(AsRef::as_ref)
            .unwrap_or_default()
            .parse::<usize>()
            .unwrap_or(3);
          #[cfg(target_os = "windows")]
          let faces = crate::os_more::net_interface::get_interfaces_simple(filter_refs)?;
          #[cfg(not(target_os = "windows"))]
          let faces = vec![];
          if count > 0 && count != faces.len() {
            return Err(format!("正确网口数量:{} 实际网口数量:{}", count, faces.len()).into());
          }

          // 创建一个异步任务列表
          let handles: Vec<_> = faces
            .iter()
            .map(|face| async move { crate::os_more::net_manage::ping(&face.ipv4, target, secs, fail_count).await })
            .collect();
          // 并发执行所有任务并处理结果
          let results = futures::future::try_join_all(handles).await?;
          // 将所有响应合并为一个字符串
          let response = results.join("\n");
          crate::p(response);
          Ok("PASS".to_string())
        }
        _ => Ok(String::new()),
      };
    }
    _ => Ok(String::new()),
  }
}
#[cfg(feature = "user")]
pub fn user_query<T: AsRef<str>>(info: &Type, args: &[T], filter: &[T]) -> e_utils::AnyResult<String> {
  let filter_refs: Vec<&str> = filter.iter().map(AsRef::as_ref).collect();
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
      let task = args.get(0).map(|x| x.as_ref()).unwrap_or_default();
      let attr_filter = args.get(1).and_then(|v| v.as_ref().parse::<u32>().ok()).filter(|&v| v > 0);
      let query_user = args.get(2).map(AsRef::as_ref);
      return match task {
        "print" => {
          let mut items = crate::os_more::desktop::get_desktop_items(query_user, attr_filter, &filter_refs);
          items.dedup_by_key(|v| v.path.clone());
          let count = items.len();
          for item in &items {
            let is_dir = if item.is_dir { "目录" } else { "" };
            let is_hidden = if item.is_hidden { "隐藏" } else { "" };
            crate::p(format!(
              "[{}] 用户[{}] 属性[{}] {} {}",
              item.path.display(),
              item.uname,
              item.attribute,
              is_dir,
              is_hidden
            ));
          }
          Ok(format!("Count: {count}"))
        }
        "nodes" => {
          let mut items = crate::os_more::desktop::get_desktop_items(query_user, attr_filter, &filter_refs);
          items.dedup_by_key(|v| v.path.clone());
          Ok(serde_json::to_string(&items)?)
        }
        _ => Ok(String::new()),
      };
    }
    _ => Ok(String::new()),
  }
}
#[cfg(feature = "system")]
pub fn system_query(info: &super::Type) -> e_utils::Result<String> {
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
      super::Type::MemoryManufacturerPartNumber => {
        system::memory_manufacturer_partnumber().map(|data| data.into_iter().map(|v| format!("{v:?}")).collect::<Vec<_>>().join(","))
      }
      _ => Ok(String::new()),
    }
  }
}
#[cfg(feature = "system")]
pub mod system {
  use e_utils::cmd::Cmd;

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
    system.cpus().first().map(|cpu| cpu.brand().to_string()).ok_or("CPU 产品名称获取失败".into())
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

  pub fn memory_manufacturer_partnumber() -> e_utils::Result<Vec<MemInfo>> {
    // 执行 WMIC 命令
    let output = Cmd::new("wmic")
      .args(&["memorychip", "get", "Manufacturer,PartNumber", "/format:csv"])
      .output()?;

    // 转换并处理输出
    let mem_list: Vec<MemInfo> = output
      .stdout
      .lines()
      .skip(1) // 跳过 CSV 表头
      .filter_map(|line| {
        let trimmed = line.trim().replace("\r", "");
        if trimmed.is_empty() {
          return None;
        }

        // 分割字段
        let fields: Vec<&str> = trimmed.split(',').collect();
        if fields.len() >= 3 {
          Some(MemInfo {
            manufacturer: fields[1].trim().to_string(),
            part_number: fields[2].trim().to_string(),
          })
        } else {
          None
        }
      })
      .collect();

    Ok(mem_list)
  }
  #[derive(Debug, PartialEq, Clone, serde::Deserialize, serde::Serialize)]
  pub struct MemInfo {
    pub manufacturer: String,
    pub part_number: String,
  }
}
