use std::{env, ffi::OsStr, path::PathBuf};

use e_utils::{cmd::Cmd, fs::AutoPath as _, regex::regex2};

use super::ty::{DriveInfo, DriveNodeInfo, DriveStatus, DriveStatusType};

pub const EDRIVE_NAME: &'static str = "devcon.exe";
/// 获取驱动目录
pub fn get_drive_path() -> PathBuf {
  env::current_dir().unwrap_or_default().to_path_buf()
}

/// 解析drivernodes数据
pub fn devcon_parse_driver_nodes(info: DriveInfo, node_data: &str) -> Vec<DriveNodeInfo> {
  let mut drive_info_list = Vec::new();
  let mut info = DriveNodeInfo::from(info);
  for _line in node_data.lines() {
    let line = _line.trim_start_matches('\x20');
    if let Some(value) = line.strip_prefix("Name: ") {
      info.name = value.to_string();
    } else if let Some(value) = line.strip_prefix("Driver node #") {
      info.drive_node = value.to_string();
    } else if let Some(value) = line.strip_prefix("Inf file is ") {
      info.inf_file = value.to_string();
    } else if let Some(value) = line.strip_prefix("Inf section is ") {
      info.inf_section = value.to_string();
    } else if let Some(value) = line.strip_prefix("Manufacturer name is ") {
      info.manufacturer_name = value.to_string();
    } else if let Some(value) = line.strip_prefix("Provider name is ") {
      info.provider_name = value.to_string();
    } else if let Some(value) = line.strip_prefix("Driver date is ") {
      info.driver_date = value.to_string();
    } else if let Some(value) = line.strip_prefix("Driver version is ") {
      info.driver_version = value.to_string();
    } else if let Some(value) = line.strip_prefix("Driver node rank is ") {
      info.driver_node_rank = value.to_string();
    } else if let Some(value) = line.strip_prefix("Driver node flags are ") {
      info.driver_node_flags = value.to_string();
      info.signed = line.contains("digitally signed");
      drive_info_list.push(info.clone());
    }
  }
  drive_info_list
}

/// # DevCon 数据处理 Vec<DriveInfo >
pub fn devcon_parse_driver_class(output: &str) -> Vec<DriveInfo> {
  let mut nline = vec![];
  for line in output.lines() {
    if line.contains(':') {
      if let Some((k, v)) = line.split_once(':') {
        let id = k.trim().to_string();
        let driver_descript = v.trim().to_string();
        nline.push(DriveInfo { id, driver_descript });
      }
    }
  }
  nline
}

/// # DevCon
/// # Example sh
/// devcon findall {* | ID [ID ...] | =class [ID [ID ...]]}
pub fn devcon<I, S>(args: I) -> e_utils::AnyResult<String>
where
  I: IntoIterator<Item = S>,
  S: AsRef<OsStr>,
{
  Ok(Cmd::new(EDRIVE_NAME).args(args).cwd(get_drive_path()).output()?.stdout)
}

/// # PnPUtil
/// # Example sh
/// ```
/// PNPUTIL [/add-driver <...> | /delete-driver <...> |
/// /export-driver <...> | /enum-drivers |
/// /enum-devices [<...>] | /enum-devicetree [<...>] |
/// /disable-device <...> | /enable-device <...> |
/// /restart-device <...> | /remove-device <...> |
/// /scan-devices [<...>] | /enum-classes [<...>] |
/// /enum-interfaces [<...>] | /enum-containers [<...>] |
/// /?]
/// ```
pub fn pnputil<I, S>(command: I) -> e_utils::AnyResult<String>
where
  I: IntoIterator<Item = S>,
  S: AsRef<OsStr>,
{
  let args = command;
  // 执行查询任务
  let res = Cmd::new("PNPUTIL").args(args).output()?.stdout;
  crate::dp(format!("PNPUTIL: {}", res));
  Ok(res)
}
/// #/enable-device 启用系统上的设备。 从 Windows 10 版本 2004 开始提供命令
/// ```
/// 从 Windows 10 版本 2004 开始可用的标志：
/// /reboot - 如果需要完成操作，请重新启动系统
/// 从 Windows 11 版本 21H2 开始可用的标志：
/// /deviceid <device ID> - 启用具有匹配设备 ID 的所有设备
/// 从 Windows 11 版本 22H2 开始可用的标志：
/// /class <name | GUID> - 按设备类名称或 GUID 进行筛选
/// /bus <name | GUID> - 按总线枚举器名称或总线类型 GUID 进行筛选
/// ```
pub fn pnputil_enable(commands: Vec<String>) -> e_utils::AnyResult<String> {
  let mut args = vec!["/enable-device".to_string()];
  args.extend(commands);
  pnputil(args)
}
/// #/disable-device 禁用系统上的设备。 从 Windows 10 版本 2004 开始提供命令
/// ```
/// 从 Windows 10 版本 2004 开始可用的标志：
/// /reboot - 如果需要完成操作，请重新启动系统
/// 从 Windows 11 版本 21H2 开始可用的标志：
/// /deviceid <device ID> - 禁用具有匹配设备 ID 的所有设备
/// 从 Windows 11 版本 22H2 开始可用的标志：
/// /class <name | GUID> - 按设备类名称或 GUID 进行筛选
/// /bus <name | GUID> - 按总线枚举器名称或总线类型 GUID 进行筛选
/// /force - 即使设备提供关键系统功能，也禁用
/// ```
pub fn pnputil_disable(commands: Vec<String>) -> e_utils::AnyResult<String> {
  let mut args = vec!["/disable-device".to_string()];
  args.extend(commands);
  pnputil(args)
}
/// #/remove-device 尝试从系统中删除设备。 从 Windows 10 版本 2004 开始提供命令。
/// ```
/// 从 Windows 10 版本 2004 开始可用的标志：
/// /subtree - 删除整个设备子树，包括任何子设备
/// /reboot - 如果需要完成操作��请重新启动系统
/// 从 Windows 11 版本 21H2 开始可用的标志：
/// /deviceid <device ID> - 删除具有匹配设备 ID 的所有设备
/// 从 Windows 11 版本 22H2 开始可用的标志：
/// /class <name | GUID> - 按设备类名称或 GUID 进行筛选
/// /bus <name | GUID> - 按总线枚举器名称或总线类型 GUID 进行筛选
/// /force - 即使设备提供关键系统功能，也会删除
/// ```
pub fn pnputil_remove(commands: Vec<String>) -> e_utils::AnyResult<String> {
  let mut args = vec!["/remove-device".to_string()];
  args.extend(commands);
  pnputil(args)
}
/// #/restart-device 尝试从系统中删除设备。 从 Windows 10 版本 2004 开始提供命令。
/// ```
///从 Windows 10 版本 2004 开始可用的标志：
/// /reboot - 如果需要完成操作，请重新启动系统
/// 从 Windows 11 版本 21H2 开始可用的标志：
/// /deviceid <device ID> - 重启具有匹配设备 ID 的所有设备
/// 从 Windows 11 版本 22H2 开始可用的标志：
/// /class <name | GUID> - 按设备类名称或 GUID 进行筛选
/// /bus <name | GUID> - 按总线枚举器名称或总线类型 GUID 进行筛选。
/// ```
pub fn pnputil_restart(commands: Vec<String>) -> e_utils::AnyResult<String> {
  let mut args = vec!["/restart-device".to_string()];
  args.extend(commands);
  pnputil(args)
}
/// #/add-driver 添加驱动程序包
/// ```
/// pnputil /add-driver c:\oem\*.inf /install
/// pnputil /add-driver x:\driver.inf /install
/// pnputil /add-driver device.inf /install
/// ```
pub fn pnputil_add_driver(commands: Vec<String>) -> e_utils::AnyResult<String> {
  let mut args = vec!["/add-driver".to_string()];
  args.extend(commands);
  pnputil(args)
}
/// 判断状态
pub fn devcon_status(id: &str) -> e_utils::AnyResult<DriveStatus> {
  let res = devcon(vec!["status", &format!("@{id}")])?;
  let mut status = DriveStatus::default();
  status.id = id.to_string();
  let status_f = |v: &str| -> DriveStatusType {
    if v.contains("disable") {
      DriveStatusType::Disabled
    } else if v.contains("running") {
      DriveStatusType::Runing
    } else {
      DriveStatusType::None
    }
  };
  if res.contains("matching device(s) found") {
    for _line in res.lines() {
      let line = _line.trim();
      if let Some(value) = line.strip_prefix("Name: ") {
        status.name = value.to_string();
      } else if let Some(value) = line.strip_prefix("Driver is ") {
        status.status = status_f(value);
        break;
      } else if let Some(value) = line.strip_prefix("Device is ") {
        status.status = status_f(value);
        break;
      }
    }
  }
  Ok(status)
}

/// #/scan-devices 扫描系统是否有任何设备硬件更改。 从 Windows 10 版本 2004 开始提供命令。
/// ```
/// /scan-devices [/instanceid <instance ID>] [/async]
/// 从 Windows 10 版本 2004 开始可用的标志：
/// /instanceid <instance ID> - 扫描设备子树中的更改
/// /async - 异步扫描更改
/// ```
pub fn pnputil_scan() -> e_utils::AnyResult<String> {
  pnputil(vec!["/scan-devices"])
}
/// #/delete-device 删除驱动程序包
/// ```
/// 删除驱动程序包
/// ```
pub fn pnputil_delete_driver<I>(commands: I) -> e_utils::AnyResult<String>
where
  I: IntoIterator<Item = String>,
{
  let mut args = vec!["/delete-driver".to_string()];
  args.extend(commands);
  pnputil(args)
}

/// #/export-driver 导出驱动
/// ```
/// pnputil /export-driver oem6.inf .
/// pnputil /export-driver * c:\backup
/// ```
pub fn pnputil_export_driver(commands: Vec<String>) -> e_utils::AnyResult<String> {
  if let Some(target) = commands.get(1) {
    let target = std::path::Path::new(target);
    target.auto_create_dir()?;
  }
  let mut args = vec!["/export-driver".to_string()];
  args.extend(commands);
  pnputil(args)
}

pub fn find_with_run<F>(args: &Vec<String>, filters: &Vec<String>, is_full: bool, f: F) -> e_utils::AnyResult<Vec<DriveStatus>>
where
  F: Fn(Vec<String>) -> e_utils::AnyResult<String>,
{
  let devcon_node_list = findnodes(&filters, is_full)?;
  let mut res_list = vec![];
  for devcon_node in &devcon_node_list {
    let mut args = args.clone();
    args.insert(0, devcon_node.id.clone());
    let _fres = f(args)?;
    crate::dp(format!("FINED: -> {}", devcon_node.id));
    let status = devcon_status(&devcon_node.id)?;
    crate::dp(format!("STATUS: {:#?}", status));
    res_list.push(status);
  }
  Ok(res_list)
}

/// 查找驱动节点
/// devcon -> https://learn.microsoft.com/zh-cn/windows-hardware/drivers/devtest/devcon-findall
pub fn findnodes(filters: &Vec<String>, is_full: bool) -> e_utils::AnyResult<Vec<DriveNodeInfo>> {
  let mut filters = filters.clone();
  let _fk = filters.get(0).cloned().unwrap_or_default();
  let fk = if _fk.starts_with("=") || _fk.starts_with("@") {
    _fk.as_str()
  } else {
    "*"
  };
  if fk != "*" {
    filters.remove(0);
  }
  // Early return for empty filters to avoid unnecessary processing
  let devcon_class = devcon_parse_driver_class(&devcon(vec!["findall", fk])?);

  // Pre-check if filters is empty to avoid repeated checks
  let filters_empty = filters.is_empty();
  let devcon_node_list = if is_full {
    devcon_class
      .iter()
      .flat_map(|dclass| {
        let nodes = match devcon_drive_node(dclass.clone()) {
          Ok(nodes) if !nodes.is_empty() => nodes,
          _ => vec![DriveNodeInfo::from(dclass.clone())],
        };
        nodes.into_iter()
      })
      .filter(|x| {
        filters_empty || {
          // Short-circuit evaluation to avoid unnecessary regex checks
          is_filter(&x.id, &filters) || is_filter(&x.driver_descript, &filters)
        }
      })
      .collect()
  } else {
    devcon_class
      .into_iter()
      .map(DriveNodeInfo::from)
      .filter(|x| filters_empty || is_filter(&x.id, &filters) || is_filter(&x.driver_descript, &filters))
      .collect()
  };

  Ok(devcon_node_list)
}
/// 请求node
pub fn devcon_drive_node(info: DriveInfo) -> e_utils::AnyResult<Vec<DriveNodeInfo>> {
  let node_res = devcon(vec!["drivernodes", &format!("@{}", info.id)])?;
  let devcon_node = devcon_parse_driver_nodes(info, &node_res);
  Ok(devcon_node)
}
/// 过滤
pub fn is_filter(data: &str, filters: &Vec<String>) -> bool {
  // Early return for empty filters
  if filters.is_empty() {
    return true;
  }
  // Use all() iterator instead of for loop
  filters.iter().all(|f| regex2(data, f).0)
}
