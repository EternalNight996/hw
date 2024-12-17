fn main() -> e_utils::AnyResult<()> {
  // 基础编译配置
  println!("cargo:rustc-env=RUSTFLAGS=-C target-cpu=native");
  println!("cargo:rustc-link-arg=-s"); // 剥离符号表
  #[cfg(feature = "built")]
  built()?;
  #[cfg(feature = "build")]
  {
    let pkg = build::from_env();
    build::build(&pkg)?;
    build::export_git_history(&pkg.version, "git_history.txt", true)?;
  }
  Ok(())
}

#[cfg(feature = "built")]
pub fn built() -> e_utils::AnyResult<()> {
  use e_utils::fs::AutoPath as _;
  let p = std::path::PathBuf::from("build.txt");
  p.auto_remove_file()?;
  built::write_built_file_with_opts(None, &p)?;
  Ok(())
}

#[cfg(feature = "build")]
mod build {
  pub use e_utils::build::*;

  // 常量定义
  const COMPANY_NAME: &str = "梦游";
  const LANG_SIMPLIFIED_CHINESE: u16 = 0x0804;
  const DEFAULT_ICON_PATH: &str = "./assets/icon.ico";
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

  #[cfg(target_os = "windows")]
  pub fn build(pkg: &e_utils::build::PackageInfo) -> e_utils::AnyResult<()> {
    // 静态链接 VC 运行时
    static_vcruntime::metabuild();
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
  #[cfg(not(target_os = "windows"))]
  pub fn build() -> e_utils::AnyResult<()> {
    Ok(())
  }
}
