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
pub async fn query_os_more(infos: &[Type], args: &Vec<String>, filter: &Vec<String>) -> e_utils::AnyResult<Vec<String>> {
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
      let res = user_query(info, &args, &filter)?;
      if !res.is_empty() {
        results.push(format!("{}={}", info, res));
      }
    }
    #[cfg(feature = "network")]
    {
      let res = network_query(info, &args, &filter).await?;
      if !res.is_empty() {
        results.push(format!("{}={}", info, res));
      }
    }
    #[cfg(feature = "drive")]
    {
      let res = drive_query(info, &args, &filter)?;
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

#[cfg(feature = "network")]
pub use _network::*;
#[cfg(feature = "network")]
mod _network {
  use super::Type;
  use sysinfo::Networks;
  /// 网络查询
  pub async fn network_query(info: &Type, args: &Vec<String>, filter: &Vec<String>) -> e_utils::AnyResult<String> {
    let task = args.get(0).map(|x| x.as_str()).unwrap_or_default();
    match info {
      Type::NetInterface => {
        return match task {
          "old" => Ok(serde_json::to_string_pretty(
            &Networks::new_with_refreshed_list().iter().map(|(k, _)| k.clone()).collect::<Vec<_>>(),
          )?),
          "all" => Ok(serde_json::to_string_pretty(&crate::os::net_interface::get_interfaces())?),
          "print" => {
            let ifaces = crate::os::net_interface::get_interfaces_simple(filter.clone());
            let count = ifaces.len();
            for iface in ifaces {
              println!("{}", serde_json::to_string_pretty(&iface)?)
            }
            Ok(format!("Count: {}", count))
          }
          "print-all" => {
            let ifaces = crate::os::net_interface::get_interfaces();
            let count = ifaces.len();
            for iface in ifaces {
              println!("{}", serde_json::to_string_pretty(&iface)?)
            }
            Ok(format!("Count: {}", count))
          }
          "check-mac" => {
            let ifaces = crate::os::net_interface::get_interfaces_simple(filter.clone());
            // Check each interface's MAC address
            for iface in &ifaces {
              let ref mac = iface.mac_addr;
              // Check against invalid MAC patterns
              if mac == "00-00-00-00-00-00" || mac == "88-88-88-88-87-88" || mac == "88-88-88-88-88-88" || mac == "TO BE FILLED BY O.E.M." {
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
          _ => Ok(serde_json::to_string_pretty(&crate::os::net_interface::get_interfaces_simple(
            filter.clone(),
          ))?),
        };
      }
      Type::NetManage => {
        return match task {
          "auto-dhcp" => {
            let mut new = vec![];
            for iface in crate::os::net_interface::get_interfaces_simple(filter.clone()) {
              let ip_res = crate::os::net_manage::set_ip_dhcp(&iface.friendly_name).await?;
              let dns_res = crate::os::net_manage::set_dns_dhcp(&iface.friendly_name).await?;
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
            Ok(crate::os::net_manage::sync_datetime(&arg).await?)
          }
          "ping" => {
            let source = args.get(1).cloned().ok_or("Args Error Source 0 ")?;
            let target = args.get(2).cloned().ok_or("Args Error Target 1 ")?;
            let count = args.get(3).cloned().ok_or("Args Error Count 2 ")?;
            Ok(crate::os::net_manage::ping(&source, &target, &count).await?)
          }
          "interfaces-ping" => {
            let target = args.get(1).cloned().ok_or("Args Error Target 1 ")?;
            let count = args.get(2).cloned().ok_or("Args Error Count 2 ")?;
            #[cfg(target_os = "windows")]
            let faces = crate::os::net_interface::get_interfaces_simple(filter.clone());
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
                async move { crate::os::net_manage::ping(&face.ipv4, &target, &count).await }
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
      _ => {}
    }
    Ok(String::new())
  }
}
#[cfg(feature = "user")]
pub use _user::*;
#[cfg(feature = "user")]
mod _user {
  use super::Type;
  use sysinfo::Users;
  pub fn user_query(info: &Type, args: &Vec<String>, filter: &Vec<String>) -> e_utils::AnyResult<String> {
    match info {
      Type::UserNames => {
        let users = Users::new_with_refreshed_list()
          .list()
          .iter()
          .map(|user| user.id().to_string())
          .collect::<Vec<String>>()
          .join(",");
        Ok(users)
      }
      Type::Desktop => {
        let task = args.get(0).map(|x| x.as_str()).unwrap_or_default();
        let attr_filter = args.get(1).and_then(|v| v.parse::<u32>().ok()).filter(|&v| v > 0);
        let query_user = args.get(2).cloned();
        return match task {
          "print" => {
            let mut items = crate::os::user_desktop::get_desktop_items(query_user, attr_filter, filter);
            items.dedup_by_key(|v| v.path.clone());
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
            Ok(serde_json::to_string_pretty(&items)?)
          }
          "all" => {
            let mut items = crate::os::user_desktop::get_desktop_items(query_user, attr_filter, filter);
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
#[cfg(feature = "system")]
use _system::*;
#[cfg(feature = "system")]
mod _system {
  use super::{bytes_to_gib, Type};
  use sysinfo::System;
  pub fn system_query(info: &Type) -> e_utils::Result<String> {
    match info {
      Type::CpuName => cpu_name(),
      Type::MemoryTotal => memory_total().map(|v| format!("{}GB", v)),
      Type::CpuCoreCount => cpu_core_count().map(|v| v.to_string()),
      Type::OsVersion => os_version(),
      Type::OsFullVersion => Ok(format!("{} {}", os_full_version()?, kernel_version()?)),
      Type::KernelVersion => kernel_version(),
      Type::HostName => host_name(),
      Type::Uptime => uptime().map(|v| format!("{}秒", v)),
      Type::CpuUsage => cpu_usage().map(|v| format!("{:.2}%", v)),
      Type::MemoryUsage => memory_usage().map(|v| format!("{:.2}%", v)),
      Type::CpuArch => cpu_arch(),
      _ => Ok(String::new()),
    }
  }
  /// 获取系统运行时间
  pub fn uptime() -> e_utils::Result<u64> {
    Ok(sysinfo::System::uptime())
  }
  /// 获取 CPU 使用率
  pub fn cpu_usage() -> e_utils::Result<f32> {
    let mut system = System::new();
    system.refresh_cpu_usage();
    Ok(system.global_cpu_usage())
  }
  /// 获取内存使用率
  pub fn memory_usage() -> e_utils::Result<f32> {
    let mut system = System::new();
    system.refresh_memory();
    Ok(system.used_memory() as f32 / system.total_memory() as f32)
  }
  /// 获取 CPU 核心数
  pub fn cpu_core_count() -> e_utils::Result<usize> {
    let mut system = System::new();
    system.refresh_cpu_frequency();
    Ok(system.cpus().len())
  }
  /// 获取 CPU 产品名称
  pub fn cpu_name() -> e_utils::Result<String> {
    let mut system = System::new();
    system.refresh_cpu_frequency();
    system
      .cpus()
      .first()
      .map(|cpu| cpu.brand().to_string())
      .ok_or("CPU 产品名称获取失败".into())
  }
  /// 获取内存总量
  pub fn memory_total() -> e_utils::Result<f64> {
    let mut system = System::new();
    system.refresh_memory();
    Ok(bytes_to_gib(system.total_memory()).round())
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

/// 将字节转换为 GiB，保留两位小数
pub fn bytes_to_gib(bytes: u64) -> f64 {
  bytes as f64 / (1024.0 * 1024.0 * 1024.0)
}

#[cfg(feature = "drive")]
use _drive::*;
#[cfg(feature = "drive")]
mod _drive {
  use crate::os::drive::*;

  use super::{bytes_to_gib, Type};
  use sysinfo::System;
  pub fn drive_query(info: &Type, args: &Vec<String>, filter: &Vec<String>) -> e_utils::AnyResult<String> {
    let task = args.get(0).map(|x| x.as_str()).unwrap_or_default();
    match info {
      Type::Drive => {
        return match task {
          "scan" => pnputil_scan(),
          "add-file" => {
            let mut outres = String::new();
            let target = args.get(1).ok_or("Args Error Target 1 ")?;
            for x in e_utils::fs::tree_folder(&target)?
              .into_iter()
              .filter(|x| std::path::Path::new(x).extension().unwrap_or_default() == "inf")
            {
              let mut new_args = filter.clone();
              if let Some(v) = new_args.get_mut(0) {
                *v = x.to_string_lossy().to_string();
              }
              let res = pnputil_add_driver(new_args)?;
              outres += &format!("{:?};;", res);
            }
            Ok(outres)
          }
          "add" => {
            let res = pnputil_add_driver(filter.clone())?;
            // 待优化判断添加驱动
            Ok(res)
          }
          "delete" => Ok(pnputil_delete_driver(args.clone())?),
          "delete-find" => {
            let commands = args.iter().map(|x| x.as_ref()).collect::<Vec<&str>>();
            let devcon_node_list = crate::drive::findnodes(&op, commands)?;
            let args_src = op.args.clone();
            let mut nodes = vec![];
            for node in &devcon_node_list {
              let inf_path: std::path::PathBuf = (&node.inf_file).into();
              let fname = inf_path.file_name().and_then(|v| v.to_str()).ok_or("File name error")?;
              let mut args = args_src.clone();
              args.insert(0, fname.to_string());
              let _dres = crate::drive::pnputil_delete_driver(args)?;
              let status = crate::drive::devcon_status(&node.id)?;
              if status.status != DriveStatusType::None {
                return Err("Delete Error: not null".into());
              }
              nodes.push(node)
            }
            Ok(serde_json::to_string_pretty(&nodes)?)
          }
          "print" => {
            let commands = op.command.iter().map(|x| x.as_ref()).collect::<Vec<&str>>();
            let devcon_node_list = crate::drive::findnodes(&op, commands)?;
            for node in &devcon_node_list {
              println!("{}", serde_json::to_string_pretty(&node)?);
            }
            Ok(String::from("PASS"))
          }
          "findnodes" => {
            let commands = op.command.iter().map(|x| x.as_ref()).collect::<Vec<&str>>();
            let devcon_node_list = crate::drive::findnodes(&op, commands)?;
            Ok(serde_json::to_string_pretty(&devcon_node_list)?)
          }
          "restart" => {
            let mut res = crate::share::default_cmd_res();
            let status_list = crate::drive::find_with_run(&op, crate::drive::pnputil_restart)?;
            for status in &status_list {
              if status.status == DriveStatusType::Runing {
                res.status = true;
                break;
              }
            }
            Ok(serde_json::to_string_pretty(&status_list)?)
          }
          "enable" => {
            let status_list = crate::drive::find_with_run(&op, crate::drive::pnputil_enable)?;
            for status in &status_list {
              if status.status != DriveStatusType::Runing {
                return Err("Enable Error: not runing".into());
              }
            }
            Ok(serde_json::to_string_pretty(&status_list)?)
          }
          "disable" => {
            let status_list = crate::drive::find_with_run(&op, crate::drive::pnputil_disable)?;
            for status in &status_list {
              if status.status != DriveStatusType::Disabled {
                return Err("Disable Error: not disabled".into());
              }
            }
            Ok(serde_json::to_string_pretty(&status_list)?)
          }
          "remove" => {
            let status_list = crate::drive::find_with_run(&op, crate::drive::pnputil_remove)?;
            for status in &status_list {
              if status.status != DriveStatusType::None {
                return Err("Remove Error: not none".into());
              }
            }
            Ok(serde_json::to_string_pretty(&status_list)?)
          }
          "export" => {
            let args_src: Vec<&str> = op.args.iter().map(|x| x.as_ref()).collect();
            Ok(crate::drive::pnputil_export_driver(args_src)?)
          }
          _ => {
            let args = op.command.clone();
            crate::drive::devcon(args)
          }
        };
      }
      _ => Ok(String::new()),
    }
  }
}
