use super::get_local_ipaddr;
use super::ty::{allocate, deallocate, htonl, Gateway, Interface, InterfaceSimple, InterfaceStatus, InterfaceType, Ipv4Net, Ipv6Net, MacAddr};
use core::ffi::c_void;
use e_utils::regex::regex2;
use libc::{c_char, strlen, wchar_t, wcslen};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use windows::Win32::{
  Foundation::{ERROR_BUFFER_OVERFLOW, NO_ERROR},
  NetworkManagement::IpHelper::{GetAdaptersAddresses, SendARP, AF_INET, AF_INET6, AF_UNSPEC, GAA_FLAG_INCLUDE_GATEWAYS, IP_ADAPTER_ADDRESSES_LH},
  Networking::WinSock::{SOCKADDR_IN, SOCKADDR_IN6},
};
///  "~Less100" => x.speed() < 100,
/// "~100" => x.speed() >= 100,
/// "~1000" => x.speed() >= 1000,
/// "~Big1000" => x.speed() >= 10000,
/// // 状态过滤
/// "~is_connected" => x.is_connected(),
/// "~has_dhcp_ip" => x.has_dhcp_ip(),
pub fn get_interfaces_simple(filter: Vec<&str>) -> e_utils::AnyResult<Vec<InterfaceSimple>> {
  // 如果没有过滤条件，直接返回所有接口
  if filter.is_empty() {
    let interfaces = get_interfaces();
    let result = interfaces.iter().map(|x| x.to_simple()).collect::<Vec<_>>();
    return if result.is_empty() { Err("No interfaces found".into()) } else { Ok(result) };
  }

  // 获取接口列表一次，避免重复调用
  let interfaces = get_interfaces();

  // 预处理过滤条件，分离特殊过滤器和正则表达式
  let mut speed_filters = Vec::new();
  let mut status_filters = Vec::new();
  let mut type_filters = Vec::new();
  let mut regex_filters = Vec::new();
  let has_auto_filter = filter.contains(&"~auto");

  // 预先计算所有接口类型的字符串表示，避免重复转换
  let interface_types: Vec<String> = interfaces.iter().map(|x| x.if_type.to_string()).collect();

  for &f in &filter {
    if f == "~auto" {
      // 已经单独处理了
      continue;
    } else if f.starts_with("~Less") || f == "~100" || f == "~1000" || f == "~Big1000" {
      // 速度过滤器
      speed_filters.push(f);
    } else if f == "~is_connected" || f == "~has_dhcp_ip" {
      // 状态过滤器
      status_filters.push(f);
    } else if interface_types.contains(&f.to_string()) {
      // 如果是接口类型，加入类型过滤器
      type_filters.push(f);
    } else {
      // 否则视为正则表达式，但不预编译
      regex_filters.push(f);
    }
  }

  // 预编译 auto 过滤器的常量
  let auto_excluded = ["vEthernet", "虚拟"];
  let auto_included = ["以太网", "Ethernet", "WLAN", "Wi-Fi", "无线", "Bluetooth", "蓝牙"];

  // 使用 with_capacity 预分配内存
  let mut res = Vec::with_capacity(interfaces.len());

  for x in &interfaces {
    let if_type_str = &interface_types[interfaces.iter().position(|i| std::ptr::eq(i, x)).unwrap()];

    // 如果有类型过滤器且匹配，则直接通过
    if !type_filters.is_empty() && type_filters.contains(&if_type_str.as_str()) {
      res.push(x.to_simple());
      continue;
    }

    let friendly_name = &x.friendly_name;

    // 单独处理 auto 过滤器（包含多个字符串比较）
    if has_auto_filter {
      if auto_excluded.iter().any(|&s| friendly_name.contains(s)) || !auto_included.iter().any(|&s| friendly_name.contains(s)) {
        continue;
      }
    }

    // 检查速度过滤器（需要计算速度）
    if !speed_filters.is_empty() {
      // 只在有速度过滤器时计算速度
      let speed = x.speed();

      let mut passes_speed = true;
      for &f in &speed_filters {
        let passes = match f {
          "~Less100" => speed < 100,
          "~100" => speed >= 100,
          "~1000" => speed >= 1000,
          "~Big1000" => speed >= 10000,
          _ => true,
        };
        if !passes {
          passes_speed = false;
          break;
        }
      }
      if !passes_speed {
        continue;
      }
    }

    // 检查状态过滤器
    if !status_filters.is_empty() {
      let mut passes_status = true;
      for &f in &status_filters {
        let passes = match f {
          "~is_connected" => x.is_connected(),
          "~has_dhcp_ip" => x.has_dhcp_ip(),
          _ => true,
        };
        if !passes {
          passes_status = false;
          break;
        }
      }
      if !passes_status {
        continue;
      }
    }

    // 如果没有正则表达式过滤器，且通过了其他过滤器，则通过
    if regex_filters.is_empty() {
      res.push(x.to_simple());
      continue;
    }

    // 最后检查正则表达式（最耗性能的操作）
    // 只要有一个正则表达式匹配就通过
    let mut passes_regex = false;
    for pattern in &regex_filters {
      if regex2(pattern, friendly_name).0 {
        passes_regex = true;
        break;
      }
    }

    if passes_regex {
      res.push(x.to_simple());
    }
  }

  if res.is_empty() {
    Err("No interfaces found".into())
  } else {
    Ok(res)
  }
}
// Get network interfaces using the IP Helper API
// Reference: https://docs.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getadaptersaddresses
pub fn get_interfaces() -> Vec<Interface> {
  let mut interfaces: Vec<Interface> = vec![];
  let mut dwsize: u32 = 2000;
  let mut mem = unsafe { allocate(dwsize as usize) } as *mut IP_ADAPTER_ADDRESSES_LH;
  let mut retries = 3;
  let mut ret_val;
  let family: u32 = AF_UNSPEC;
  let flags: u32 = GAA_FLAG_INCLUDE_GATEWAYS;
  loop {
    let old_size = dwsize as usize;
    ret_val = unsafe { GetAdaptersAddresses(family, flags, std::ptr::null_mut::<std::ffi::c_void>(), mem, &mut dwsize) };
    if ret_val != ERROR_BUFFER_OVERFLOW || retries <= 0 {
      break;
    }
    unsafe { deallocate(mem as *mut u8, old_size as usize) };
    mem = unsafe { allocate(dwsize as usize) as *mut IP_ADAPTER_ADDRESSES_LH };
    retries -= 1;
  }
  if ret_val == NO_ERROR {
    // Enumerate all adapters
    let mut cur = mem;
    while !cur.is_null() {
      let if_type: u32 = unsafe { (*cur).IfType };
      // Index
      let anon1 = unsafe { (*cur).Anonymous1 };
      let anon = unsafe { anon1.Anonymous };
      let index = anon.IfIndex;
      // Flags
      let anon2 = unsafe { (*cur).Anonymous2 };
      let flags = unsafe { anon2.Flags };
      // Name
      let p_aname = unsafe { (*cur).AdapterName.0 };
      let aname_len = unsafe { strlen(p_aname as *const c_char) };
      let aname_slice = unsafe { std::slice::from_raw_parts(p_aname, aname_len) };
      let adapter_name = String::from_utf8(aname_slice.to_vec()).unwrap_or_default();
      // Friendly Name
      let p_fname = unsafe { (*cur).FriendlyName.0 };
      let fname_len = unsafe { wcslen(p_fname as *const wchar_t) };
      let fname_slice = unsafe { std::slice::from_raw_parts(p_fname, fname_len) };
      let friendly_name = String::from_utf16(fname_slice).unwrap_or_default();
      // Description
      let p_desc = unsafe { (*cur).Description.0 };
      let desc_len = unsafe { wcslen(p_desc as *const wchar_t) };
      let desc_slice = unsafe { std::slice::from_raw_parts(p_desc, desc_len) };
      let description = String::from_utf16(desc_slice).unwrap_or_default();
      // MAC address
      let mac_addr_arr: [u8; 6] = unsafe { (*cur).PhysicalAddress }[..6].try_into().unwrap_or([0, 0, 0, 0, 0, 0]);
      let mac_addr: MacAddr = MacAddr::new(mac_addr_arr);
      // TransmitLinkSpeed (bits per second)
      let transmit_speed = unsafe { (*cur).TransmitLinkSpeed };
      // ReceiveLinkSpeed (bits per second)
      let receive_speed = unsafe { (*cur).ReceiveLinkSpeed };
      let mut ipv4_vec: Vec<Ipv4Net> = vec![];
      let mut ipv6_vec: Vec<Ipv6Net> = vec![];
      // Enumerate all IPs
      let mut cur_a = unsafe { (*cur).FirstUnicastAddress };
      while !cur_a.is_null() {
        let addr = unsafe { (*cur_a).Address };
        let prefix_len = unsafe { (*cur_a).OnLinkPrefixLength };
        let sockaddr = unsafe { *addr.lpSockaddr };
        if sockaddr.sa_family == AF_INET as u16 {
          let sockaddr: *mut SOCKADDR_IN = addr.lpSockaddr as *mut SOCKADDR_IN;
          let a = unsafe { (*sockaddr).sin_addr.S_un.S_addr };
          let ipv4 = if cfg!(target_endian = "little") {
            Ipv4Addr::from(a.swap_bytes())
          } else {
            Ipv4Addr::from(a)
          };
          let ipv4_net: Ipv4Net = Ipv4Net::new(ipv4, prefix_len);
          ipv4_vec.push(ipv4_net);
        } else if sockaddr.sa_family == AF_INET6 as u16 {
          let sockaddr: *mut SOCKADDR_IN6 = addr.lpSockaddr as *mut SOCKADDR_IN6;
          let a = unsafe { (*sockaddr).sin6_addr.u.Byte };
          let ipv6 = Ipv6Addr::from(a);
          let ipv6_net: Ipv6Net = Ipv6Net::new(ipv6, prefix_len);
          ipv6_vec.push(ipv6_net);
        }
        cur_a = unsafe { (*cur_a).Next };
      }
      // Gateway
      // TODO: IPv6 support
      let mut gateway_ips: Vec<Ipv4Addr> = vec![];
      let mut cur_g = unsafe { (*cur).FirstGatewayAddress };
      while !cur_g.is_null() {
        let addr = unsafe { (*cur_g).Address };
        let sockaddr = unsafe { *addr.lpSockaddr };
        if sockaddr.sa_family == AF_INET as u16 {
          let sockaddr: *mut SOCKADDR_IN = addr.lpSockaddr as *mut SOCKADDR_IN;
          let a = unsafe { (*sockaddr).sin_addr.S_un.S_addr };
          let ipv4 = if cfg!(target_endian = "little") {
            Ipv4Addr::from(a.swap_bytes())
          } else {
            Ipv4Addr::from(a)
          };
          gateway_ips.push(ipv4);
        }
        cur_g = unsafe { (*cur_g).Next };
      }
      let default_gateway: Option<Gateway> = match gateway_ips.get(0) {
        Some(gateway_ip) => {
          if let Some(ip_net) = ipv4_vec.get(0) {
            let mac_addr = get_mac_through_arp(ip_net.addr, *gateway_ip);
            let gateway = Gateway {
              mac_addr: mac_addr,
              ip_addr: IpAddr::V4(*gateway_ip),
            };
            Some(gateway)
          } else {
            None
          }
        }
        None => None,
      };
      // 获取 DNS 服务器地址
      let mut dns_servers: Vec<IpAddr> = vec![];
      let mut cur_dns = unsafe { (*cur).FirstDnsServerAddress };
      while !cur_dns.is_null() {
        let addr = unsafe { (*cur_dns).Address };
        let sockaddr = unsafe { *addr.lpSockaddr };

        if sockaddr.sa_family == AF_INET as u16 {
          let sockaddr: *mut SOCKADDR_IN = addr.lpSockaddr as *mut SOCKADDR_IN;
          let a = unsafe { (*sockaddr).sin_addr.S_un.S_addr };
          let ipv4 = if cfg!(target_endian = "little") {
            Ipv4Addr::from(a.swap_bytes())
          } else {
            Ipv4Addr::from(a)
          };
          dns_servers.push(IpAddr::V4(ipv4));
        } else if sockaddr.sa_family == AF_INET6 as u16 {
          let sockaddr: *mut SOCKADDR_IN6 = addr.lpSockaddr as *mut SOCKADDR_IN6;
          let a = unsafe { (*sockaddr).sin6_addr.u.Byte };
          let ipv6 = Ipv6Addr::from(a);
          dns_servers.push(IpAddr::V6(ipv6));
        }

        cur_dns = unsafe { (*cur_dns).Next };
      }

      let interface: Interface = Interface {
        index,
        name: adapter_name,
        friendly_name: friendly_name,
        description: description,
        if_type: InterfaceType::from(if_type),
        mac_addr: mac_addr,
        ipv4: ipv4_vec,
        ipv6: ipv6_vec,
        flags,
        transmit_speed: transmit_speed,
        receive_speed: receive_speed,
        gateway: default_gateway,
        oper_status: InterfaceStatus::from(unsafe { (*cur).OperStatus }),
        dns_servers,
      };
      interfaces.push(interface);
      cur = unsafe { (*cur).Next };
    }
  } else {
    unsafe {
      deallocate(mem as *mut u8, dwsize as usize);
    }
  }
  unsafe {
    deallocate(mem as *mut u8, dwsize as usize);
  }
  return interfaces;
}

fn get_mac_through_arp(src_ip: Ipv4Addr, dst_ip: Ipv4Addr) -> MacAddr {
  let src_ip_int: u32 = htonl(u32::from(src_ip));
  let dst_ip_int: u32 = htonl(u32::from(dst_ip));
  let mut out_buf_len: u32 = 6;
  let mut target_mac_addr: [u8; 6] = [0; 6];
  let res = unsafe { SendARP(dst_ip_int, src_ip_int, target_mac_addr.as_mut_ptr() as *mut c_void, &mut out_buf_len) };
  if res == NO_ERROR {
    MacAddr::new(target_mac_addr)
  } else {
    MacAddr::zero()
  }
}

pub fn get_default_gateway_macaddr() -> [u8; 6] {
  match get_default_gateway() {
    Ok(gateway) => gateway.mac_addr.octets(),
    Err(_) => MacAddr::zero().octets(),
  }
}

/// Get default Gateway
pub fn get_default_gateway() -> Result<Gateway, String> {
  let local_ip: IpAddr = match get_local_ipaddr() {
    Ok(local_ip) => local_ip,
    Err(_) => return Err(String::from("Local IP address not found")),
  };
  let interfaces: Vec<Interface> = get_interfaces();
  for iface in interfaces {
    match local_ip {
      IpAddr::V4(local_ipv4) => {
        if iface.ipv4.iter().any(|x| x.addr == local_ipv4) {
          if let Some(gateway) = iface.gateway {
            return Ok(gateway);
          }
        }
      }
      IpAddr::V6(local_ipv6) => {
        if iface.ipv6.iter().any(|x| x.addr == local_ipv6) {
          if let Some(gateway) = iface.gateway {
            return Ok(gateway);
          }
        }
      }
    }
  }
  Err(String::from("Default Gateway not found"))
}
