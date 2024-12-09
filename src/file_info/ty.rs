use std::path::{Path, PathBuf};

use goblin::elf::header::*;
use goblin::mach::cputype::*;
use goblin::pe::header::*;
use serde::{Deserialize, Serialize};
use strum::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Type {
  pub fname: String,
  pub cwd: Option<String>,
  pub exe_type: ExeType,
  pub architecture: ArchType,
  pub platform: PlatformType,
  pub is_lib: bool,
  pub is_64: bool,
  pub libs: Vec<Dependency>,
}
impl Default for Type {
  fn default() -> Self {
    Type {
      cwd: Some("#origin#/plugins".to_string()),
      fname: Default::default(),
      exe_type: Default::default(),
      architecture: Default::default(),
      platform: Default::default(),
      is_lib: Default::default(),
      is_64: Default::default(),
      libs: Default::default(),
    }
  }
}
#[derive(Default, Clone, Debug, Deserialize, Serialize)]
pub struct ImportedFunction {
  pub name: String,
  pub ordinal: u16,
  pub rva: usize,
}

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
pub struct Dependency {
  pub name: String,
  pub is_exists: bool,
  pub fullpath: Option<PathBuf>,
  pub functions: Vec<ImportedFunction>,
}
/// Operating System Platform
#[derive(Default, Clone, Copy, Debug, Display, PartialEq, VariantArray, EnumString, Deserialize, Serialize)]
#[repr(u8)]
pub enum PlatformType {
  /// Microsoft Windows
  Windows,
  /// Apple macOS
  MacOS,
  /// Linux-based systems
  Linux,
  /// Google Android
  Android,
  /// Apple iOS
  IOS,
  /// Unix-based systems
  Unix,
  ///
  #[default]
  #[strum(to_string = "未知")]
  Unknown,
}
impl PlatformType {
  pub fn is_current(&self) -> bool {
    match self {
      PlatformType::Windows => cfg!(target_os = "windows"),
      PlatformType::MacOS => cfg!(target_os = "macos"),
      PlatformType::Linux => cfg!(target_os = "linux"),
      PlatformType::Android => cfg!(target_os = "android"),
      PlatformType::IOS => cfg!(target_os = "ios"),
      PlatformType::Unix => cfg!(unix),
      PlatformType::Unknown => true, // We assume Unknown is compatible with any platform
    }
  }
}

/// Application type enumeration
#[derive(Default, Clone, Copy, Debug, Display, PartialEq, EnumString, VariantArray, Deserialize, Serialize)]
#[repr(u8)]
pub enum ExeType {
  /// Windows executable file
  #[strum(to_string = "Windows Executable")]
  Exe,
  /// DOS program file
  #[strum(to_string = "DOS Program")]
  Com,
  /// Batch file
  #[strum(to_string = "Batch File")]
  Bat,
  /// Cmd file
  #[default]
  #[strum(to_string = "Cmd Command")]
  Cmd,
  /// Cmd file
  #[strum(to_string = "Cmd File")]
  CmdFile,
  /// Visual Basic Script file
  #[strum(to_string = "VBScript")]
  Vbs,
  /// PowerShell script file
  #[strum(to_string = "PowerShell Script")]
  Ps1,
  /// Dynamic Link Library file
  #[strum(to_string = "Dynamic Link Library")]
  Dll,
  /// Shell script file
  #[strum(to_string = "Shell Script")]
  ShellScript,
  /// macOS application bundle
  #[strum(to_string = "macOS Application")]
  MacOSApp,
  /// Linux Executable
  #[strum(to_string = "Linux Executable")]
  LinuxExecutable,
  /// Linux Shared Object
  #[strum(to_string = "Linux Shared Object")]
  LinuxSharedObject,
  /// Linux Relocatable Object
  #[strum(to_string = "Linux Relocatable Object")]
  LinuxRelocatable,
  /// Linux Core Dump
  #[strum(to_string = "Linux Core Dump")]
  LinuxCoreDump,
  /// Android Executable
  #[strum(to_string = "Android Executable")]
  AndroidExecutable,
  /// Android Shared Object
  #[strum(to_string = "Android Shared Object")]
  AndroidSharedObject,
  /// Android Application Package
  #[strum(to_string = "Android Application Package")]
  AndroidApk,
  /// iOS Application
  #[strum(to_string = "iOS Application")]
  IosApp,
  /// Other application types
  #[strum(to_string = "未知")]
  Unknown,
}
impl ExeType {
  pub fn from_linux(t: u16, is_lib: bool) -> Self {
    match t {
      goblin::elf::header::ET_EXEC => ExeType::LinuxExecutable,
      goblin::elf::header::ET_DYN => {
        if is_lib {
          ExeType::LinuxSharedObject
        } else {
          ExeType::LinuxExecutable
        }
      }
      goblin::elf::header::ET_REL => ExeType::LinuxRelocatable,
      goblin::elf::header::ET_CORE => ExeType::LinuxCoreDump,
      _ => ExeType::Unknown,
    }
  }
  pub fn from_android(t: u16, is_lib: bool) -> Self {
    match t {
      goblin::elf::header::ET_EXEC => ExeType::AndroidExecutable,
      goblin::elf::header::ET_DYN => {
        if is_lib {
          ExeType::AndroidSharedObject
        } else {
          ExeType::AndroidExecutable // 可能是位置无关的可执行文件
        }
      }
      _ => ExeType::Unknown,
    }
  }
  pub fn from_target(p: impl AsRef<Path>) -> Self {
    match p
      .as_ref()
      .extension()
      .and_then(|x| x.to_str())
      .unwrap_or_default()
      .to_lowercase()
      .as_str()
    {
      "exe" => ExeType::Exe,
      "dll" => ExeType::Dll,
      "com" => ExeType::Com,
      "bat" => ExeType::Bat,
      "cmd" => ExeType::CmdFile,
      "vbs" => ExeType::Vbs,
      "ps1" => ExeType::Ps1,
      "sh" => ExeType::ShellScript,
      "app" => ExeType::MacOSApp,
      "apk" => ExeType::AndroidApk,
      "ipa" => ExeType::IosApp,
      "so" => ExeType::LinuxSharedObject,
      "dylib" => ExeType::Dll,
      _ => ExeType::Unknown,
    }
  }
  pub fn to_extension(&self) -> &'static str {
    match self {
      ExeType::Exe => "exe",
      ExeType::Dll => "dll",
      ExeType::Com => "com",
      ExeType::Bat => "bat",
      ExeType::CmdFile => "cmd",
      ExeType::Vbs => "vbs",
      ExeType::Ps1 => "ps1",
      ExeType::ShellScript => "sh",
      ExeType::MacOSApp => "app",
      ExeType::AndroidApk => "apk",
      ExeType::IosApp => "ipa",
      ExeType::LinuxSharedObject => "so",
      ExeType::LinuxRelocatable => "o",
      ExeType::LinuxCoreDump => "core",
      ExeType::AndroidSharedObject => "so",
      ExeType::LinuxExecutable => "",
      ExeType::AndroidExecutable => "",
      ExeType::Cmd => "",
      ExeType::Unknown => "",
    }
  }
}
/// Instruction Set Architecture (ISA)
#[derive(Default, Clone, Copy, Debug, Display, PartialEq, VariantArray, EnumString, Deserialize, Serialize)]
#[repr(u8)]
pub enum ArchType {
  /// x86 architecture (32-bit)
  X86,
  /// x86-64 architecture (64-bit)
  X86_64,
  /// ARM architecture (32-bit)
  ARM,
  /// ARM64 architecture (64-bit)
  ARM64,
  /// MIPS architecture
  MIPS,
  /// PowerPC architecture (32-bit)
  PowerPC,
  /// PowerPC64 architecture (64-bit)
  PowerPC64,
  /// SPARC architecture
  SPARC,
  /// MC68000 architecture
  MC68K,
  /// HP PA-RISC architecture
  HPPA,
  /// MC88000 architecture
  MC88K,
  /// Intel i860 architecture
  I860,
  /// Alpha architecture
  Alpha,
  /// VAX architecture
  VAX,
  /// ARM64_32 architecture
  ARM64_32,
  #[default]
  #[strum(to_string = "未知")]
  Unknown,
}
impl ArchType {
  pub fn is_current(&self) -> bool {
    match self {
      ArchType::X86 => cfg!(target_arch = "x86"),
      ArchType::X86_64 => cfg!(target_arch = "x86_64"),
      ArchType::ARM => cfg!(target_arch = "arm"),
      ArchType::ARM64 => cfg!(target_arch = "aarch64"),
      ArchType::MIPS => cfg!(target_arch = "mips"),
      ArchType::PowerPC => cfg!(target_arch = "powerpc"),
      ArchType::PowerPC64 => cfg!(target_arch = "powerpc64"),
      ArchType::SPARC => cfg!(target_arch = "sparc"),
      ArchType::MC68K => cfg!(target_arch = "m68k"),
      ArchType::HPPA => false,
      ArchType::MC88K => false,    // Rust doesn't have a specific target for MC88K
      ArchType::I860 => false,     // Rust doesn't have a specific target for I860
      ArchType::Alpha => false,    // Rust doesn't have a specific target for Alpha
      ArchType::VAX => false,      // Rust doesn't have a specific target for VAX
      ArchType::ARM64_32 => false, // Rust doesn't have a specific target for ARM64_32
      ArchType::Unknown => true,   // We assume Unknown is compatible with any architecture
    }
  }
  pub fn from_mach(value: u32) -> Self {
    match value {
      CPU_TYPE_X86 => ArchType::X86,
      CPU_TYPE_X86_64 => ArchType::X86_64,
      CPU_TYPE_ARM => ArchType::ARM,
      CPU_TYPE_ARM64 => ArchType::ARM64,
      CPU_TYPE_MIPS => ArchType::MIPS,
      CPU_TYPE_POWERPC => ArchType::PowerPC,
      CPU_TYPE_POWERPC64 => ArchType::PowerPC64,
      CPU_TYPE_SPARC => ArchType::SPARC,
      CPU_TYPE_MC680X0 => ArchType::MC68K,
      CPU_TYPE_HPPA => ArchType::HPPA,
      CPU_TYPE_MC88000 => ArchType::MC88K,
      CPU_TYPE_I860 => ArchType::I860,
      CPU_TYPE_ALPHA => ArchType::Alpha,
      CPU_TYPE_VAX => ArchType::VAX,
      CPU_TYPE_ARM64_32 => ArchType::ARM64_32,
      _ => ArchType::Unknown,
    }
  }
  pub fn from_pe(value: u16) -> Self {
    match value {
      COFF_MACHINE_X86 => ArchType::X86,
      COFF_MACHINE_X86_64 => ArchType::X86_64,
      COFF_MACHINE_ARM => ArchType::ARM,
      COFF_MACHINE_ARM64 => ArchType::ARM64,
      COFF_MACHINE_POWERPC => ArchType::PowerPC,
      COFF_MACHINE_ALPHA => ArchType::Alpha,
      _ => ArchType::Unknown,
    }
  }
  pub fn from_elf(value: u16) -> Self {
    match value {
      EM_386 => ArchType::X86,
      EM_X86_64 => ArchType::X86_64,
      EM_ARM => ArchType::ARM,
      EM_AARCH64 => ArchType::ARM64,
      EM_MIPS => ArchType::MIPS,
      EM_PPC => ArchType::PowerPC,
      EM_PPC64 => ArchType::PowerPC64,
      EM_SPARC => ArchType::SPARC,
      _ => ArchType::Unknown,
    }
  }
}
