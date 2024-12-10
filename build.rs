#![allow(unused)]
use e_utils::AnyResult as Result;

// 常量定义
const COMPANY_NAME: &str = "梦游";
const LANG_SIMPLIFIED_CHINESE: u16 = 0x0804;
const DEFAULT_ICON_PATH: &str = "./assets/icon.ico";
const BUILD_FILE_PATH: &str = "build.txt";
const GIT_HISTORY_FILE_PATH: &str = "git_history.txt";

fn main() -> Result<()> {
  // 基础编译配置
  println!("cargo:rustc-env=RUSTFLAGS=-C target-cpu=native");
  println!("cargo:rustc-link-arg=-s"); // 剥离符号表
  #[cfg(all(feature = "build", target_os = "windows"))]
  setup_windows_build()?;
  #[cfg(feature = "built")]
  {
    use e_utils::fs::AutoPath as _;
    let p = std::path::PathBuf::from(BUILD_FILE_PATH);
    p.auto_remove_file()?;
    built::write_built_file_with_opts(None, &p)?;
  }
  #[cfg(feature = "build")]
  e_utils::build::export_git_history(GIT_HISTORY_FILE_PATH, true)?;

  Ok(())
}

#[cfg(all(feature = "build", target_os = "windows"))]
fn setup_windows_build() -> Result<()> {
  use e_utils::build::PackageInfo;
  /// 从环境变量中获取包信息
  pub fn from_env() -> e_utils::build::PackageInfo {
    e_utils::build::PackageInfo {
      name: env!("CARGO_PKG_NAME").to_string(),
      version: env!("CARGO_PKG_VERSION").to_string(),
      description: env!("CARGO_PKG_DESCRIPTION").to_string(),
      authors: env!("CARGO_PKG_AUTHORS").to_string(),
      homepage: env!("CARGO_PKG_HOMEPAGE").to_string(),
      repository: env!("CARGO_PKG_REPOSITORY").to_string(),
    }
  }

  // 静态链接 VC 运行时
  static_vcruntime::metabuild();
  let pkg = from_env();

  winresource::WindowsResource::new()
    .set("FileDescription", &pkg.description)
    .set("ProductName", &pkg.name)
    .set("ProductVersion", &pkg.version)
    .set("FileVersion", &pkg.version)
    .set("OriginalFilename", &format!("{}.exe", pkg.name))
    .set("InternalName", &pkg.name)
    .set("CompanyName", COMPANY_NAME)
    .set("LegalCopyright", &pkg.copyright())
    .set("Comments", &pkg.comments())
    .set("Homepage", &pkg.homepage)
    .set_icon(DEFAULT_ICON_PATH)
    .set_language(LANG_SIMPLIFIED_CHINESE)
    .compile()?;

  Ok(())
}
