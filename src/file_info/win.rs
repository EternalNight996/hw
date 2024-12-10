use e_utils::cmd::ExeType;
use goblin::elf::Elf;
use goblin::pe::PE;
use goblin::Object;
use std::borrow::Cow;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::{env, fs};
use winapi::shared::minwindef::DWORD;
use winapi::um::processenv::SearchPathW;

use super::{ArchType, Dependency, ExeTypeEx, ImportedFunction, PlatformType};

/// 寻找完整路径
pub fn find_path(dll_name: &str) -> Option<std::path::PathBuf> {
  #[cfg(not(any(target_os = "windows", target_os = "linux")))]
  let fullpath = None;
  #[cfg(target_os = "windows")]
  let fullpath = super::find_dll_path(dll_name);
  #[cfg(target_os = "linux")]
  let fullpath = find_so_path(dll_name);
  fullpath
}

/// 寻找完整路径
pub fn find_so_path(so_name: &str) -> Option<PathBuf> {
  // 1. 检查 LD_LIBRARY_PATH
  if let Ok(ld_library_path) = env::var("LD_LIBRARY_PATH") {
    for dir in ld_library_path.split(':') {
      let path = Path::new(dir).join(so_name);
      if path.exists() {
        return Some(path);
      }
    }
  }

  // 2. 检查 /etc/ld.so.cache
  // 注意：这需要 root 权限，所以我们跳过这一步

  // 3. 检查默认路径 /lib 和 /usr/lib
  for dir in ["/lib", "/usr/lib"] {
    let path = Path::new(dir).join(so_name);
    if path.exists() {
      return Some(path);
    }
  }

  // 4. 检查 /etc/ld.so.conf 中列出的目录
  if let Ok(contents) = fs::read_to_string("/etc/ld.so.conf") {
    for line in contents.lines() {
      if line.starts_with('/') {
        let path = Path::new(line).join(so_name);
        if path.exists() {
          return Some(path);
        }
      }
    }
  }

  None
}

pub fn lib_copy(target: impl AsRef<Path>, to: impl AsRef<Path>) -> e_utils::AnyResult<usize> {
  let mut count = 0;
  let to = to.as_ref();
  for lib in lib_data_parse(&fs::read(target)?)? {
    if let Some(p) = lib.fullpath {
      crate::p(format!("{} -> {}", p.display(), to.display()));
      let pto = to.join(p.file_name().unwrap_or_default());
      e_utils::fs::auto_copy(p, &pto)?;
      count += 1;
    }
  }
  Ok(count)
}
pub fn open(target: impl AsRef<Path>) -> e_utils::AnyResult<super::Type> {
  let t = target.as_ref();
  let mut res = data_parse(&fs::read(t)?)?;
  // 提取文件名
  res.fname = t.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
  // 提取当前工作目录
  res.cwd = t.parent().and_then(|p| p.to_str().map(|x| x.to_string()));
  res.exe_type = ExeType::from_target(target);
  Ok(res)
}
pub async fn a_open(target: impl AsRef<Path>) -> e_utils::AnyResult<super::Type> {
  let t = target.as_ref();
  let mut res = data_parse(&tokio::fs::read(t).await?)?;
  // 提取文件名
  res.fname = t.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
  // 提取当前工作目录
  res.cwd = t.parent().and_then(|p| p.to_str().map(|x| x.to_string()));
  res.exe_type = ExeType::from_target(target);
  Ok(res)
}
pub fn lib_data_parse(buffer: &Vec<u8>) -> e_utils::AnyResult<Vec<Dependency>> {
  Ok(match Object::parse(&buffer)? {
    Object::Elf(elf) => elf_lib_data_parse(elf),
    Object::PE(pe) => pe_lib_data_parse(pe),
    _ => return Err("Cannot parse lib data".into()),
  })
}
pub fn data_parse(buffer: &Vec<u8>) -> Result<super::Type, goblin::error::Error> {
  let mut slf = super::Type::default();
  match Object::parse(&buffer)? {
    Object::Elf(elf) => {
      slf.platform = PlatformType::Linux;
      slf.architecture = ArchType::from_elf(elf.header.e_machine);
      // 检查是否为 Android ELF
      let is_android = elf.libraries.iter().any(|lib| lib.to_lowercase().contains("android"));
      if is_android {
        slf.platform = PlatformType::Android;
        slf.exe_type = ExeTypeEx::from_android(elf.header.e_type, elf.is_lib).0;
      } else {
        slf.platform = PlatformType::Linux;
        slf.exe_type = ExeTypeEx::from_linux(elf.header.e_type, elf.is_lib).0;
      }
      slf.is_64 = elf.is_64;
      slf.is_lib = elf.is_lib;
      // 添加 ELF 依赖分析
      slf.libs = elf_lib_data_parse(elf);
    }
    Object::PE(pe) => {
      slf.platform = PlatformType::Windows;
      slf.architecture = ArchType::from_pe(pe.header.coff_header.machine);
      slf.exe_type = if pe.is_lib { ExeType::Dll } else { ExeType::WindowsExe };
      slf.is_64 = pe.is_64;
      slf.is_lib = pe.is_lib;
      slf.libs = pe_lib_data_parse(pe);
    }
    Object::Mach(mach) => {
      // Mach-O 可能是 macOS 或 iOS
      match mach {
        goblin::mach::Mach::Binary(macho) => {
          slf.architecture = ArchType::from_mach(macho.header.cputype());
          slf.platform = match slf.architecture {
            ArchType::ARM64_32 | ArchType::ARM | ArchType::ARM64 => PlatformType::IOS,
            _ => PlatformType::MacOS,
          };
          slf.exe_type = ExeType::MacOSApp;
          slf.is_64 = macho.is_64;
        }
        goblin::mach::Mach::Fat(fat) => {
          for arch in fat.iter_arches() {
            if let Ok(arch) = arch {
              let t = ArchType::from_mach(arch.cputype());
              if t != ArchType::Unknown {
                slf.architecture = t;
                slf.is_64 = arch.is_64();
                break;
              }
            }
          }
          slf.platform = PlatformType::MacOS; // Assume Fat binary is macOS
          slf.exe_type = ExeType::MacOSApp;
        }
      }
    }
    Object::COFF(coff) => {
      slf.platform = PlatformType::Windows;
      slf.architecture = ArchType::from_pe(coff.header.machine);
      slf.exe_type = ExeType::Unknown; // COFF files can be various types, we need more info to determine
    }
    Object::Archive(archive) => {
      // Try to determine Archive type and architecture
      if let Some(first_member) = archive.members().get(0) {
        if let Ok(inner_object) = Object::parse(first_member.as_bytes()) {
          match inner_object {
            Object::Elf(_) => {
              slf.platform = PlatformType::Linux;
              slf.exe_type = ExeType::So;
            }
            Object::PE(_) | Object::COFF(_) => {
              slf.platform = PlatformType::Windows;
              slf.exe_type = ExeType::Dll;
            }
            Object::Mach(_) => {
              slf.platform = PlatformType::MacOS;
              slf.exe_type = ExeType::Dll;
            }
            _ => {
              slf.platform = PlatformType::Unknown;
              slf.exe_type = ExeType::Unknown;
            }
          }
          // Try to determine architecture from the first member
          slf.architecture = match inner_object {
            Object::Elf(elf) => ArchType::from_elf(elf.header.e_machine),
            Object::PE(pe) => ArchType::from_pe(pe.header.coff_header.machine),
            Object::Mach(mach) => {
              if let goblin::mach::Mach::Binary(macho) = mach {
                ArchType::from_mach(macho.header.cputype())
              } else {
                ArchType::Unknown
              }
            }
            _ => ArchType::Unknown,
          };
        }
      }
      slf.is_lib = true; // Archives are typically libraries
    }
    _ => {}
  }
  Ok(slf)
}
fn elf_lib_data_parse(elf: Elf<'_>) -> Vec<Dependency> {
  // 添加 ELF 依赖分析
  let mut libs = HashMap::new();
  for lib in elf.libraries {
    let entry = libs.entry(lib.to_string()).or_insert_with(|| {
      #[cfg(not(any(target_os = "windows", target_os = "linux")))]
      let fullpath = None;
      #[cfg(target_os = "windows")]
      let fullpath = None; // ELF 文件在 Windows 上不适用
      #[cfg(target_os = "linux")]
      let fullpath = super::find::find_so_path(lib);
      Dependency {
        name: lib.to_string(),
        functions: Vec::new(),
        is_exists: fullpath.is_some(),
        fullpath,
      }
    });
    // 分析导入函数
    for sym in elf.dynsyms.iter() {
      if sym.st_type() == goblin::elf::sym::STT_FUNC && sym.st_shndx == 0 {
        if let Some(name) = elf.dynstrtab.get_at(sym.st_name) {
          entry.functions.push(ImportedFunction {
            name: name.to_string(),
            ordinal: 0, // ELF 不使用序数
            rva: sym.st_value as usize,
          });
        }
      }
    }
  }
  libs.into_values().collect()
}

fn pe_lib_data_parse(pe: PE<'_>) -> Vec<Dependency> {
  let mut libs = HashMap::new();
  // Detailed dependency analysis for PE
  for import in pe.imports {
    let entry = libs.entry(import.dll.to_string()).or_insert_with(|| {
      let fullpath = super::find_dll_path(import.dll);
      Dependency {
        name: import.dll.to_string(),
        functions: Vec::new(),
        is_exists: fullpath.is_some(),
        fullpath,
      }
    });

    let function = ImportedFunction {
      name: match import.name {
        Cow::Borrowed(s) => s.to_string(),
        Cow::Owned(s) => s,
      },
      ordinal: import.ordinal,
      rva: import.rva,
    };
    entry.functions.push(function);
  }
  libs.into_values().collect()
}

/// 寻找完整路径
pub fn find_dll_path(dll_name: &str) -> Option<std::path::PathBuf> {
  let wide_dll_name: Vec<u16> = OsStr::new(dll_name).encode_wide().chain(std::iter::once(0)).collect();

  let mut buffer = [0u16; 260]; // MAX_PATH
  let mut copied: DWORD = 0;

  unsafe {
    let result = SearchPathW(
      std::ptr::null(),
      wide_dll_name.as_ptr(),
      std::ptr::null(),
      buffer.len() as DWORD,
      buffer.as_mut_ptr(),
      std::ptr::null_mut(),
    );

    if result > 0 {
      copied = result;
    }
  }

  if copied > 0 {
    let path = String::from_utf16_lossy(&buffer[..copied as usize]);
    Some(PathBuf::from(path))
  } else {
    None
  }
}
