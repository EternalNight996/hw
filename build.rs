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
  use e_utils::fs::AutoPath as _;
  // 常量定义
  const COMPANY_NAME: &str = "梦游";
  const LANG_SIMPLIFIED_CHINESE: u16 = 0x0804;
  const DEFAULT_ICON_PATH: &str = "./assets/icon.ico";
  /// 导出git历史
  pub fn export_git_history(pkg_version: &str, target: impl AsRef<std::path::Path>, is_china: bool) -> e_utils::AnyResult<()> {
    target.as_ref().auto_remove_file()?;
    let build_time = if is_china {
      e_utils::chrono::china_now()
        .ok_or("获取时间失败")?
        .format(e_utils::chrono::STANDARD_DATETIME_FORMAT)
    } else {
      e_utils::chrono::now().format(e_utils::chrono::STANDARD_DATETIME_FORMAT)
    };
    let tag_commits = e_utils::build::git_tag_commits().unwrap_or_default();
    let version_info = format!(
      "当前版本号: {}\n\
      构建时间: {}\n\
      构建类型: {}\n\
      历史提交 :\n\n{}",
      pkg_version,
      build_time,
      if cfg!(debug_assertions) { "Debug" } else { "Release" },
      tag_commits,
    );
    e_utils::fs::write_utf8(&target.as_ref().to_path_buf(), &version_info, false, false)?;

    Ok(())
  }
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
