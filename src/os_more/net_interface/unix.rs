use super::ty::InterfaceSimple;

/// 枚举网络接口(Linux)
/// 注:getifaddrs 枚举尚未实现;Windows 走 win.rs(IpHelper)。
/// 返回明确错误,上层(如 etest 系统信息页)以 unwrap_or_default 兜底为空列表。
pub fn get_interfaces_simple(_filter: Vec<&str>) -> e_utils::AnyResult<Vec<InterfaceSimple>> {
  Err("net-interface: Linux 接口枚举未实现".into())
}
