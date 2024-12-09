use std::mem;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, UdpSocket};

use serde::{Deserialize, Serialize};
use strum::*;

/// Structure of Network Interface information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Interface {
  /// Index of network interface
  pub index: u32,
  /// Name of network interface
  pub name: String,
  /// Friendly Name of network interface
  pub friendly_name: String,
  /// Description of the network interface
  pub description: String,
  /// Interface Type
  pub if_type: InterfaceType,
  /// MAC address of network interface
  pub mac_addr: MacAddr,
  /// List of Ipv4Net for the network interface
  pub ipv4: Vec<Ipv4Net>,
  /// List of Ipv6Net for the network interface
  pub ipv6: Vec<Ipv6Net>,
  /// Flags for the network interface (OS Specific)
  pub flags: u32,
  /// Speed in bits per second of the transmit for the network interface
  pub transmit_speed: u64,
  /// Speed in bits per second of the receive for the network interface
  pub receive_speed: u64,
  /// Default gateway for the network interface
  pub gateway: Option<Gateway>,
  /// Operational status of the network interface
  pub oper_status: InterfaceStatus,
  /// DNS servers for the network interface
  pub dns_servers: Vec<IpAddr>,
}

impl Interface {
  pub fn is_connected(&self) -> bool {
    self.oper_status == InterfaceStatus::Up
  }
  /// 获取接口速率（以 Mbps 为单位）
  pub fn speed(&self) -> u64 {
    // 如果发送和接收速率都存在且相等，返回其中一个
    let (tx, rx) = (self.transmit_speed, self.receive_speed);
    if tx == rx {
      return tx / 1_000_000; // 转换为 Mbps
    }
    // 否则返回发送速率（如果存在）
    tx / 1_000_000
  }
  /// 检查网络接口是否已获取DHCP分配的IP
  pub fn has_dhcp_ip(&self) -> bool {
    // 1.必须连接
    if self.is_connected() {
      // 2. 必须有网关
      // 3. 必须有IP地址
      // 4. IP地址不能是特殊地址（如回环、链路本地等）
      if let Some(gateway) = &self.gateway {
        let has_valid_ip = !self.ipv4.iter().any(|x| {
          let ref v = x.addr;
          v.is_link_local() || v.is_broadcast() || v.is_documentation() || v.is_loopback() || v.is_multicast() || v.is_unspecified()
        }) || !self.ipv6.iter().any(|ip| ip.addr.is_loopback());

        // 检查网关是否有效
        let has_valid_gateway = !gateway.ip_addr.is_loopback() && !gateway.ip_addr.is_unspecified();

        has_valid_ip && has_valid_gateway
      } else {
        false
      }
    } else {
      false
    }
  }
  pub fn has_ip(&self) -> bool {
    !self.ipv4.is_empty() || !self.ipv6.is_empty()
  }

  /// 获取更详细的网络状态（包括DHCP状态）
  pub fn network_status(&self) -> String {
    if self.is_connected() {
      if self.has_dhcp_ip() {
        format!("已连接 - DHCP - {}", format!("速率: {} Mbps", self.speed()))
      } else if self.has_ip() {
        "已连接 - 已设置静态IP".to_string()
      } else {
        "已连接 - 无法获取IP".to_string()
      }
    } else {
      "未连接".to_string()
    }
  }
  pub fn to_simple(&self) -> InterfaceSimple {
    let ipv4s: Vec<_> = self.ipv4.iter().map(|x| x.addr.to_string()).collect();
    let ipv6s: Vec<_> = self.ipv6.iter().map(|x| x.addr.to_string()).collect();
    InterfaceSimple {
      index: self.index,
      name: self.name.clone(),
      friendly_name: self.friendly_name.clone(),
      description: self.description.clone(),
      if_type: self.if_type.clone(),
      if_type_name: self.if_type.get_message().unwrap_or_default().to_string(),
      mac_addr: self.mac_addr.to_string().to_ascii_uppercase(),
      ipv4: ipv4s.join(","),
      ipv6: ipv6s.join(","),
      flags: self.flags,
      gateway: self.gateway.as_ref().map(|x| x.ip_addr.to_string()),
      speed_mb: self.speed(),
      has_dhcp_ip: self.has_dhcp_ip(),
      network_status: self.network_status(),
      oper_status: self.oper_status,
      is_connected: self.is_connected(),
      dns_servers: self.dns_servers.iter().map(|x| x.to_string()).collect(),
    }
  }
}
/// Structure of Network Interface information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InterfaceSimple {
  /// Index of network interface
  pub index: u32,
  /// Name of network interface
  pub name: String,
  /// Friendly Name of network interface
  pub friendly_name: String,
  /// Description of the network interface
  pub description: String,
  /// Interface Type
  pub if_type: InterfaceType,
  /// Interface Type Name
  pub if_type_name: String,
  /// MAC address of network interface
  pub mac_addr: String,
  /// List of Ipv4Net for the network interface
  pub ipv4: String,
  /// List of Ipv6Net for the network interface
  pub ipv6: String,
  /// Flags for the network interface (OS Specific)
  pub flags: u32,
  /// Default gateway for the network interface
  pub gateway: Option<String>,
  /// Speed in Mbps of the network interface
  pub speed_mb: u64,
  /// Whether the network interface has a DHCP IP
  pub has_dhcp_ip: bool,
  /// Network status of the network interface
  pub network_status: String,
  /// Operational status of the network interface
  pub oper_status: InterfaceStatus,
  /// Whether the network interface is connected
  pub is_connected: bool,
  /// DNS servers for the network interface
  pub dns_servers: Vec<String>,
}

/// Structure of default Gateway information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Gateway {
  /// MAC address of Gateway
  pub mac_addr: MacAddr,
  /// IP address of Gateway
  pub ip_addr: IpAddr,
}

impl Gateway {
  /// Construct a new Gateway instance
  pub fn new() -> Gateway {
    Gateway {
      mac_addr: MacAddr::zero(),
      ip_addr: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
    }
  }
}


/// Get IP address of the default Network Interface
pub fn get_local_ipaddr() -> Result<IpAddr, String> {
  let socket = match UdpSocket::bind("0.0.0.0:0") {
    Ok(s) => s,
    Err(e) => return Err(String::from(e.to_string())),
  };
  if let Err(e) = socket.connect("1.1.1.1:80") {
    return Err(String::from(e.to_string()));
  };
  match socket.local_addr() {
    Ok(addr) => Ok(addr.ip()),
    Err(e) => return Err(String::from(e.to_string())),
  }
}

/// 网络接口状态
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Display, EnumProperty, EnumIter, EnumMessage)]
pub enum InterfaceStatus {
  /// 接口已连接
  #[strum(message = "已连接")]
  #[strum(props(windows = "1", unix = "1", macos = "1"))]
  Up,
  /// 接口已断开
  #[strum(message = "已断开")]
  #[strum(props(windows = "2", unix = "2", macos = "2"))]
  Down,
  /// 接口测试中
  #[strum(message = "测试中")]
  #[strum(props(windows = "3", unix = "0", macos = "0"))]
  Testing,
  /// 接口状态未知
  #[strum(message = "未知")]
  #[strum(props(windows = "4", unix = "0", macos = "0"))]
  Unknown,
  /// 接口休眠
  #[strum(message = "休眠")]
  #[strum(props(windows = "5", unix = "0", macos = "0"))]
  Dormant,
  /// 接口不存在
  #[strum(message = "不存在")]
  #[strum(props(windows = "6", unix = "0", macos = "0"))]
  NotPresent,
  /// 底层接口已断开
  #[strum(message = "底层断开")]
  #[strum(props(windows = "7", unix = "0", macos = "0"))]
  LowerLayerDown,
}
impl InterfaceStatus {
  /// Returns OS-specific value of InterfaceStatus
  #[cfg(target_os = "windows")]
  pub fn value(&self) -> i32 {
    self.get_str("windows").and_then(|s| s.parse().ok()).unwrap_or(0)
  }
}

#[cfg(target_os = "windows")]
impl From<i32> for InterfaceStatus {
  fn from(v: i32) -> Self {
    for variant in Self::iter() {
      if let Some(value_str) = variant.get_str("windows") {
        if let Ok(value) = value_str.parse::<i32>() {
          if value == v {
            return variant;
          }
        }
      }
    }
    InterfaceStatus::Unknown
  }
}
/// 网络接口类型
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Display, EnumMessage, EnumProperty, EnumIter)]
pub enum InterfaceType {
  /// 未知接口类型
  #[strum(message = "未知({0})")]
  #[strum(props(windows = "1", unix = "0", macos = "0"))]
  Unknown(u32),
  /// 使用以太网连接的网络接口
  #[strum(message = "以太网")]
  #[strum(props(windows = "6", unix = "1", macos = "6"))]
  Ethernet,
  /// 使用令牌环的网络接口
  #[strum(message = "令牌环")]
  #[strum(props(windows = "9", unix = "4", macos = "9"))]
  TokenRing,
  /// 使用光纤分布式数据接口(FDDI)连接的网络接口
  #[strum(message = "光纤分布式数据接口")]
  #[strum(props(windows = "15", unix = "774", macos = "15"))]
  Fddi,
  /// 使用点对点协议(PPP)连接的网络接口
  #[strum(message = "点对点协议")]
  #[strum(props(windows = "23", unix = "512", macos = "23"))]
  Ppp,
  /// 环回接口
  #[strum(message = "环回")]
  #[strum(props(windows = "24", unix = "772", macos = "24"))]
  Loopback,
  /// 3兆比特以太网
  #[strum(message = "3兆比特以太网")]
  #[strum(props(windows = "26", unix = "2", macos = "26"))]
  Ethernet3Megabit,
  /// 串行线路网际协议
  #[strum(message = "串行线路网际协议")]
  #[strum(props(windows = "28", unix = "256", macos = "28"))]
  Slip,
  /// 异步传输模式
  #[strum(message = "异步传输模式")]
  #[strum(props(windows = "37", unix = "19", macos = "37"))]
  Atm,
  /// 通用调制解调器
  #[strum(message = "通用调制解调器")]
  #[strum(props(windows = "48", unix = "48", macos = "48"))]
  GenericModem,
  /// 快速以太网T
  #[strum(message = "快速以太网T")]
  #[strum(props(windows = "62", unix = "62", macos = "62"))]
  FastEthernetT,
  /// ISDN
  #[strum(message = "ISDN")]
  #[strum(props(windows = "63", unix = "63", macos = "63"))]
  Isdn,
  /// 快速以太网FX
  #[strum(message = "快速以太网FX")]
  #[strum(props(windows = "69", unix = "69", macos = "69"))]
  FastEthernetFx,
  /// 无线网络
  #[strum(message = "无线网络")]
  #[strum(props(windows = "71", unix = "801", macos = "71"))]
  Wireless80211,
  /// 非对称数字用户线路
  #[strum(message = "非对称数字用户线路")]
  #[strum(props(windows = "94", unix = "94", macos = "94"))]
  AsymmetricDsl,
  /// 速率自适应数字用户线路
  #[strum(message = "速率自适应数字用户线路")]
  #[strum(props(windows = "95", unix = "95", macos = "95"))]
  RateAdaptDsl,
  /// 对称数字用户线路
  #[strum(message = "对称数字用户线路")]
  #[strum(props(windows = "96", unix = "96", macos = "96"))]
  SymmetricDsl,
  /// 超高速数字用户线路
  #[strum(message = "超高速数字用户线路")]
  #[strum(props(windows = "97", unix = "97", macos = "97"))]
  VeryHighSpeedDsl,
  /// IP over ATM
  #[strum(message = "IP over ATM")]
  #[strum(props(windows = "114", unix = "114", macos = "114"))]
  IPOverAtm,
  /// 千兆以太网
  #[strum(message = "千兆以太网")]
  #[strum(props(windows = "117", unix = "117", macos = "117"))]
  GigabitEthernet,
  /// 隧道
  #[strum(message = "隧道")]
  #[strum(props(windows = "131", unix = "768", macos = "131"))]
  Tunnel,
  /// 多速率对称数字用户线路
  #[strum(message = "多速率对称数字用户线路")]
  #[strum(props(windows = "143", unix = "143", macos = "143"))]
  MultiRateSymmetricDsl,
  /// 高性能串行总线
  #[strum(message = "高性能串行总线")]
  #[strum(props(windows = "144", unix = "144", macos = "144"))]
  HighPerformanceSerialBus,
  /// 无线城域网
  #[strum(message = "无线城域网")]
  #[strum(props(windows = "237", unix = "237", macos = "237"))]
  Wman,
  /// 无线广域网PP
  #[strum(message = "无线广域网PP")]
  #[strum(props(windows = "243", unix = "243", macos = "243"))]
  Wwanpp,
  /// 无线广域网PP2
  #[strum(message = "无线广域网PP2")]
  #[strum(props(windows = "244", unix = "244", macos = "244"))]
  Wwanpp2,
}
impl InterfaceType {
  /// Returns OS-specific value of InterfaceStatus
  #[cfg(target_os = "windows")]
  pub fn value(&self) -> i32 {
    self.get_str("windows").and_then(|s| s.parse().ok()).unwrap_or(0)
  }
}

#[cfg(target_os = "windows")]
impl From<u32> for InterfaceType {
  fn from(v: u32) -> Self {
    for variant in Self::iter() {
      if let Some(value_str) = variant.get_str("windows") {
        if let Ok(value) = value_str.parse::<u32>() {
          if value == v {
            return variant;
          }
        }
      }
    }
    InterfaceType::Unknown(v)
  }
}

/// Structure of IPv4 Network
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ipv4Net {
  /// IPv4 Address
  pub addr: Ipv4Addr,
  /// Prefix Length
  pub prefix_len: u8,
  /// Network Mask
  pub netmask: Ipv4Addr,
}

impl Ipv4Net {
  /// Construct a new Ipv4Net instance from IPv4 Address and Prefix Length
  pub fn new(ipv4_addr: Ipv4Addr, prefix_len: u8) -> Ipv4Net {
    Ipv4Net {
      addr: ipv4_addr,
      prefix_len: prefix_len,
      netmask: prefix_to_ipv4_netmask(prefix_len),
    }
  }
  /// Construct a new Ipv4Net instance from IPv4 Address and Network Mask
  pub fn new_with_netmask(ipv4_addr: Ipv4Addr, netmask: Ipv4Addr) -> Ipv4Net {
    Ipv4Net {
      addr: ipv4_addr,
      prefix_len: ipv4_netmask_to_prefix(netmask),
      netmask: netmask,
    }
  }
}

/// Structure of IPv6 Network
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ipv6Net {
  /// IPv6 Address
  pub addr: Ipv6Addr,
  /// Prefix Length
  pub prefix_len: u8,
  /// Network Mask
  pub netmask: Ipv6Addr,
}

impl Ipv6Net {
  /// Construct a new Ipv6Net instance from IPv6 Address and Prefix Length
  pub fn new(ipv6_addr: Ipv6Addr, prefix_len: u8) -> Ipv6Net {
    Ipv6Net {
      addr: ipv6_addr,
      prefix_len: prefix_len,
      netmask: prefix_to_ipv6_netmask(prefix_len),
    }
  }
  /// Construct a new Ipv6Net instance from IPv6 Address and Network Mask
  pub fn new_with_netmask(ipv6_addr: Ipv6Addr, netmask: Ipv6Addr) -> Ipv6Net {
    Ipv6Net {
      addr: ipv6_addr,
      prefix_len: ipv6_netmask_to_prefix(netmask),
      netmask: netmask,
    }
  }
}

fn ipv4_netmask_to_prefix(netmask: Ipv4Addr) -> u8 {
  let netmask = u32::from(netmask);
  let prefix = (!netmask).leading_zeros() as u8;
  if (u64::from(netmask) << prefix) & 0xffff_ffff != 0 {
    0
  } else {
    prefix
  }
}

fn ipv6_netmask_to_prefix(netmask: Ipv6Addr) -> u8 {
  let netmask = netmask.segments();
  let mut mask_iter = netmask.iter();
  let mut prefix = 0;
  for &segment in &mut mask_iter {
    if segment == 0xffff {
      prefix += 16;
    } else if segment == 0 {
      break;
    } else {
      let prefix_bits = (!segment).leading_zeros() as u8;
      if segment << prefix_bits != 0 {
        return 0;
      }
      prefix += prefix_bits;
      break;
    }
  }
  for &segment in mask_iter {
    if segment != 0 {
      return 0;
    }
  }
  prefix
}

fn prefix_to_ipv4_netmask(prefix_len: u8) -> Ipv4Addr {
  let netmask_u32: u32 = u32::max_value().checked_shl(32 - prefix_len as u32).unwrap_or(0);
  Ipv4Addr::from(netmask_u32)
}

fn prefix_to_ipv6_netmask(prefix_len: u8) -> Ipv6Addr {
  let netmask_u128: u128 = u128::max_value().checked_shl((128 - prefix_len) as u32).unwrap_or(u128::min_value());
  Ipv6Addr::from(netmask_u128)
}

#[cfg(target_endian = "little")]
pub fn htonl(val: u32) -> u32 {
  let o3 = (val >> 24) as u8;
  let o2 = (val >> 16) as u8;
  let o1 = (val >> 8) as u8;
  let o0 = val as u8;
  (o0 as u32) << 24 | (o1 as u32) << 16 | (o2 as u32) << 8 | (o3 as u32)
}

#[cfg(target_endian = "big")]
pub fn htonl(val: u32) -> u32 {
  val
}

/// Structure of MAC address
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MacAddr(u8, u8, u8, u8, u8, u8);

impl MacAddr {
  /// Construct a new MacAddr instance from the given octets
  pub fn new(octets: [u8; 6]) -> MacAddr {
    MacAddr(octets[0], octets[1], octets[2], octets[3], octets[4], octets[5])
  }
  /// Returns an array of MAC address octets
  pub fn octets(&self) -> [u8; 6] {
    [self.0, self.1, self.2, self.3, self.4, self.5]
  }
  /// Return a formatted string of MAC address
  pub fn address(&self) -> String {
    format!(
      "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
      self.0, self.1, self.2, self.3, self.4, self.5
    )
  }
  /// Construct an all-zero MacAddr instance
  pub fn zero() -> MacAddr {
    MacAddr(0, 0, 0, 0, 0, 0)
  }
  /// Construct a new MacAddr instance from a colon-separated string of hex format
  pub fn from_hex_format(hex_mac_addr: &str) -> MacAddr {
    if hex_mac_addr.len() != 17 {
      return MacAddr(0, 0, 0, 0, 0, 0);
    }
    let fields: Vec<&str> = hex_mac_addr.split(":").collect();
    let o1: u8 = u8::from_str_radix(&fields[0], 0x10).unwrap_or(0);
    let o2: u8 = u8::from_str_radix(&fields[1], 0x10).unwrap_or(0);
    let o3: u8 = u8::from_str_radix(&fields[2], 0x10).unwrap_or(0);
    let o4: u8 = u8::from_str_radix(&fields[3], 0x10).unwrap_or(0);
    let o5: u8 = u8::from_str_radix(&fields[4], 0x10).unwrap_or(0);
    let o6: u8 = u8::from_str_radix(&fields[5], 0x10).unwrap_or(0);
    MacAddr(o1, o2, o3, o4, o5, o6)
  }
}

impl std::fmt::Display for MacAddr {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let _ = write!(
      f,
      "{:<02x}-{:<02x}-{:<02x}-{:<02x}-{:<02x}-{:<02x}",
      self.0, self.1, self.2, self.3, self.4, self.5
    );
    Ok(())
  }
}

#[allow(dead_code)]
#[inline]
pub(crate) unsafe fn allocate(size: usize) -> *mut u8 {
  ptr_from_vec(Vec::with_capacity(size))
}
#[inline]
fn ptr_from_vec(mut buf: Vec<u8>) -> *mut u8 {
  let ptr = buf.as_mut_ptr();
  mem::forget(buf);
  ptr
}

#[allow(dead_code)]
#[inline]
pub(crate) unsafe fn deallocate(ptr: *mut u8, old_size: usize) {
  Vec::from_raw_parts(ptr, 0, old_size);
}
#[allow(dead_code)]
pub(crate) fn empty() -> *mut u8 {
  1 as *mut u8
}
#[allow(dead_code)]
pub(crate) unsafe fn reallocate(ptr: *mut u8, old_size: usize, new_size: usize) -> *mut u8 {
  if old_size > new_size {
    let mut buf = Vec::from_raw_parts(ptr, new_size, old_size);
    buf.shrink_to_fit();

    ptr_from_vec(buf)
  } else if new_size > old_size {
    let additional = new_size - old_size;

    let mut buf = Vec::from_raw_parts(ptr, 0, old_size);
    buf.reserve_exact(additional);

    ptr_from_vec(buf)
  } else {
    ptr
  }
}
