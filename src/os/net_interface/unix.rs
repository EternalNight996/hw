// use libc as c;
// use std::ffi::CStr;
// use std::io;
// use std::mem;
// use std::net::IpAddr;
// use std::net::Ipv4Addr;
// use std::net::Ipv6Addr;
// use std::ptr;
// use std::ptr::NonNull;
// use super::r#type::Interface;

// #[cfg(any(target_os = "android", target_os = "linux"))]
// use crate::linux::*;

// // Yes, wrong for Solaris's vile offspring. Don't complain, send patches.
// #[cfg(not(any(target_os = "android", target_os = "linux")))]
// use crate::bsd::*;


// /// Returns an iterator that produces the list of interfaces that the
// /// operating system considers "up", that is, configured and active.
// pub fn interfaces() -> io::Result<InterfaceUp> {
//   let mut base = ptr::null_mut();

//   if 0 != unsafe { c::getifaddrs(&mut base) } {
//     return Err(io::Error::last_os_error());
//   }

//   let base = NonNull::new(base);
//   let iter = Iter(base);

//   Ok(InterfaceUp { base, iter })
// }

// pub struct InterfaceUp {
//   base: Option<NonNull<c::ifaddrs>>,
//   iter: Iter,
// }

// impl Iterator for InterfaceUp {
//   type Item = Interface;

//   fn next(&mut self) -> Option<Self::Item> {
//     self.iter.find_map(|curr| to_interface(self.base, curr))
//   }
// }

// impl Drop for InterfaceUp {
//   fn drop(&mut self) {
//     if let Some(mut base) = self.base {
//       unsafe { c::freeifaddrs(base.as_mut()) };
//     }
//   }
// }

// struct Iter(Option<NonNull<c::ifaddrs>>);

// impl Iterator for Iter {
//   type Item = NonNull<c::ifaddrs>;

//   fn next(&mut self) -> Option<Self::Item> {
//     let curr = self.0?;
//     let next = unsafe { curr.as_ref().ifa_next };
//     mem::replace(&mut self.0, NonNull::new(next))
//   }
// }

// fn ip(addr: NonNull<c::sockaddr>) -> Option<IpAddr> {
//   let family = unsafe { addr.as_ref().sa_family };

//   match family as _ {
//     c::AF_INET => {
//       let addr = unsafe { &*(addr.as_ptr() as *mut c::sockaddr_in) };
//       let addr = Ipv4Addr::from(u32::from_be(addr.sin_addr.s_addr));
//       Some(IpAddr::V4(addr))
//     }
//     c::AF_INET6 => {
//       let addr = unsafe { &*(addr.as_ptr() as *mut c::sockaddr_in6) };
//       let [b0, b1, b2, b3, b4, b5, b6, b7, b8, b9, b10, b11, b12, b13, b14, b15] =
//         addr.sin6_addr.s6_addr;
//       let s0 = 256 * b0 as u16 + b1 as u16;
//       let s1 = 256 * b2 as u16 + b3 as u16;
//       let s2 = 256 * b4 as u16 + b5 as u16;
//       let s3 = 256 * b6 as u16 + b7 as u16;
//       let s4 = 256 * b8 as u16 + b9 as u16;
//       let s5 = 256 * b10 as u16 + b11 as u16;
//       let s6 = 256 * b12 as u16 + b13 as u16;
//       let s7 = 256 * b14 as u16 + b15 as u16;
//       let addr = Ipv6Addr::new(s0, s1, s2, s3, s4, s5, s6, s7);
//       Some(IpAddr::V6(addr))
//     }
//     _ => None,
//   }
// }

// fn to_interface(base: Option<NonNull<c::ifaddrs>>, curr: NonNull<c::ifaddrs>) -> Option<Interface> {
//   let curr = unsafe { curr.as_ref() };
//   let addr = NonNull::new(curr.ifa_addr)?;

//   if is_link(addr) {
//     return None;
//   }

//   let address = ip(addr)?;
//   let netmask = NonNull::new(curr.ifa_netmask).and_then(ip)?;

//   let name = unsafe { CStr::from_ptr(curr.ifa_name) };
//   let mac = Iter(base)
//     .find_map(|link| mac_of(name, link))
//     .unwrap_or_default();
//   let name = name.to_string_lossy().into_owned();

//   let flags = From::from(curr.ifa_flags);

//   let scope_id = address.is_ipv6().then(|| {
//     let addr = addr.as_ptr() as *const c::sockaddr_in6;
//     unsafe { (*addr).sin6_scope_id }
//   });

//   Some(Interface {
//     name,
//     flags,
//     mac,
//     address,
//     scope_id,
//     netmask,
//   })
// }

// #[cfg(any(target_os = "android", target_os = "linux"))]
// mod linux {
//   use libc as c;
//   use std::ffi::CStr;
//   use std::ptr::NonNull;

//   pub(crate) fn is_link(addr: NonNull<c::sockaddr>) -> bool {
//     c::AF_PACKET == unsafe { addr.as_ref().sa_family } as _
//   }

//   pub(crate) fn mac_of(name: &CStr, link: NonNull<c::ifaddrs>) -> Option<[u8; 6]> {
//     let link = unsafe { link.as_ref() };
//     let addr = NonNull::new(link.ifa_addr)?;

//     if !is_link(addr) {
//       return None;
//     }

//     let ok = unsafe { CStr::from_ptr(link.ifa_name) }
//       .to_bytes()
//       .strip_prefix(name.to_bytes())
//       .filter(|suffix| suffix.is_empty() || suffix.starts_with(b":"))
//       .is_some();

//     if !ok {
//       return None;
//     }

//     let addr = link.ifa_addr as *const _ as *const c::sockaddr_ll;
//     let addr = unsafe { &*addr };

//     if addr.sll_halen != 6 {
//       return None;
//     }

//     let [b0, b1, b2, b3, b4, b5, _, _] = addr.sll_addr;

//     Some([b0, b1, b2, b3, b4, b5])
//   }
// }


// #[cfg(all(unix, not(any(target_os = "android", target_os = "linux"))))]
// mod bsd {
//     use libc as c;
//     use std::ffi::CStr;
//     use std::ptr::NonNull;

//     pub(crate) fn is_link(addr: NonNull<c::sockaddr>) -> bool {
//         c::AF_LINK == unsafe { addr.as_ref().sa_family } as _
//     }

//     pub(crate) fn mac_of(
//         name: &CStr,
//         link: NonNull<c::ifaddrs>,
//     ) -> Option<[u8; 6]> {
//         let link = unsafe { link.as_ref() };
//         let addr = NonNull::new(link.ifa_addr)?;

//         if !is_link(addr) {
//             return None;
//         }

//         let ok = unsafe { CStr::from_ptr(link.ifa_name) }
//             .to_bytes()
//             .strip_prefix(name.to_bytes())
//             .filter(|suffix| suffix.is_empty() || suffix.starts_with(b":"))
//             .is_some();

//         if !ok {
//             return None;
//         }

//         let addr = link.ifa_addr as *const _ as *const c::sockaddr_dl;
//         let addr = unsafe { &*addr };

//         if addr.sdl_alen != 6 {
//             return None;
//         }

//         // sdl data contains both the if name and link-level address.
//         // See: https://illumos.org/man/3socket/sockaddr_dl
//         let start = addr.sdl_nlen as usize; // length of the if name.
//         let end = start + addr.sdl_alen as usize;
//         let data = unsafe {
//             std::slice::from_raw_parts(
//                 &addr.sdl_data as *const _ as *const u8,
//                 end,
//             )
//         };

//         if let [b0, b1, b2, b3, b4, b5] = data[start..end] {
//             Some([b0, b1, b2, b3, b4, b5])
//         } else {
//             None
//         }
//     }
// }

// pub fn get_default_gateway_macaddr() -> [u8; 6] {
//     MacAddr::zero().octets()
//   }