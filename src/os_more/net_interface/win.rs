use core::ffi::c_void;
use e_utils::regex::regex2;
use libc::{c_char, strlen, wchar_t, wcslen};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use windows::Win32::{
  Foundation::{ERROR_BUFFER_OVERFLOW, NO_ERROR},
  NetworkManagement::IpHelper::{GetAdaptersAddresses, SendARP, AF_INET, AF_INET6, AF_UNSPEC, GAA_FLAG_INCLUDE_GATEWAYS, IP_ADAPTER_ADDRESSES_LH},
  Networking::WinSock::{SOCKADDR_IN, SOCKADDR_IN6},
};

use super::ty::{allocate, deallocate, htonl, Gateway, Interface, InterfaceSimple, InterfaceStatus, InterfaceType, Ipv4Net, Ipv6Net, MacAddr};

use super::get_local_ipaddr;
///  "~Less100" => x.speed() < 100,
/// "~100" => x.speed() >= 100,
/// "~1000" => x.speed() >= 1000,
/// "~Big1000" => x.speed() >= 10000,
/// // 状态过滤
/// "~is_connected" => x.is_connected(),
/// "~has_dhcp_ip" => x.has_dhcp_ip(),
pub fn get_interfaces_simple(filter: Vec<&str>) -> e_utils::AnyResult<Vec<InterfaceSimple>> {
  // 如果没有过滤条件，直接返回所有接口
  let res: Vec<InterfaceSimple> = if filter.is_empty() {
    get_interfaces().iter().map(|x| x.to_simple()).collect()
  } else {
    get_interfaces()
      .iter()
      .filter(|x| {
        filter.iter().all(|f| match *f {
          // 速度过滤（从小到大排序，避免重复计算）
          "~Less100" => x.speed() < 100,
          "~100" => x.speed() >= 100,
          "~1000" => x.speed() >= 1000,
          "~Big1000" => x.speed() >= 10000,
          // 状态过滤
          "~is_connected" => x.is_connected(),
          "~has_dhcp_ip" => x.has_dhcp_ip(),
          // 类型匹配和正则匹配
          f => f == &x.if_type.to_string() || regex2(&x.friendly_name, f).0,
        })
      })
      .map(|x| x.to_simple())
      .collect()
  };
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
