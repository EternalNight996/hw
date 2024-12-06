use e_utils::{fs::AutoPath, AnyResult as Result};

// 常量定义
const COMPANY_NAME: &str = "梦游";
const LANG_SIMPLIFIED_CHINESE: u16 = 0x0804;
const DEFAULT_ICON_PATH: &str = "./assets/icon.ico";
const BUILD_FILE_PATH: &str = "build.txt";
const VERSION_FILE_PATH: &str = "version.txt";

// 包信息结构体
#[derive(Debug)]
struct PackageInfo {
  name: String,
  version: String,
  description: String,
  authors: String,
  homepage: String,
  repository: String,
}

impl PackageInfo {
  fn from_env() -> Self {
    Self {
      name: env!("CARGO_PKG_NAME").to_string(),
      version: env!("CARGO_PKG_VERSION").to_string(),
      description: env!("CARGO_PKG_DESCRIPTION").to_string(),
      authors: env!("CARGO_PKG_AUTHORS").to_string(),
      homepage: env!("CARGO_PKG_HOMEPAGE").to_string(),
      repository: env!("CARGO_PKG_REPOSITORY").to_string(),
    }
  }

  fn formatted_authors(&self) -> String {
    self.authors.replace(':', ", ")
  }
}

fn main() -> Result<()> {
  // 基础编译配置
  println!("cargo:rustc-env=RUSTFLAGS=-C target-cpu=native");
  println!("cargo:rustc-link-arg=-s"); // 剥离符号表

  #[cfg(all(feature = "build", target_os = "windows"))]
  setup_windows_build()?;
  #[cfg(all(feature = "build", feature = "built"))]
  {
    let p = std::path::PathBuf::from(BUILD_FILE_PATH);
    p.auto_remove_file()?;
    built::write_built_file_with_opts(None, &p)?;
  }
  #[cfg(all(feature = "build"))]
  setup_build_version()?;

  Ok(())
}

#[cfg(all(feature = "build", target_os = "windows"))]
fn setup_windows_build() -> Result<()> {
  use e_utils::{chrono::Datelike as _, system::chrono};

  // 静态链接 VC 运行时
  static_vcruntime::metabuild();
  let pkg_info = PackageInfo::from_env();

  let copyright = format!(
    "版权所有 © {} {}",
    chrono::Local::now().year(),
    if pkg_info.authors.is_empty() {
      String::new()
    } else {
      pkg_info.formatted_authors()
    }
  );

  let additional_info = format!(
    "版本: {}\n主页: {}\n仓库: {}\n作者: {}",
    pkg_info.version,
    pkg_info.homepage,
    pkg_info.repository,
    pkg_info.formatted_authors()
  );

  winresource::WindowsResource::new()
    .set("FileDescription", &pkg_info.description)
    .set("ProductName", &pkg_info.name)
    .set("ProductVersion", &pkg_info.version)
    .set("FileVersion", &pkg_info.version)
    .set("OriginalFilename", &format!("{}.exe", pkg_info.name))
    .set("InternalName", &pkg_info.name)
    .set("CompanyName", COMPANY_NAME)
    .set("LegalCopyright", &copyright)
    .set("Comments", &additional_info)
    .set("Homepage", &pkg_info.homepage)
    .set_icon(DEFAULT_ICON_PATH)
    .set_language(LANG_SIMPLIFIED_CHINESE)
    .compile()?;

  Ok(())
}
fn setup_build_version() -> Result<()> {
  use e_utils::{chrono::STANDARD_DATETIME_FORMAT, fs::write_utf8};
  use std::path::PathBuf;
  let p = PathBuf::from(VERSION_FILE_PATH);
  p.auto_remove_file()?;
  let pkg_info = PackageInfo::from_env();
  let git_info = get_git_info();
  let build_time = e_utils::chrono::china_now().ok_or("获取时间失败")?.format(STANDARD_DATETIME_FORMAT);
  let tag_commits = get_tag_commits().unwrap_or_default();
  let version_info = format!(
    "版本号: {}\n\
       Git信息:\n\
       - 提交哈希: {}\n\
       - 提交信息: {}\n\
       - 提交作者: {}\n\
       - 提交时间: {}\n\
       - 标签提交: {}\n\
       \n\
       构建时间: {}\n\
       构建类型: {}\n",
    pkg_info.version,
    git_info.commit_hash,
    git_info.commit_message,
    git_info.commit_author,
    git_info.commit_date,
    tag_commits,
    build_time,
    if cfg!(debug_assertions) { "Debug" } else { "Release" }
  );
  write_utf8(&p, &version_info, false, false)?;

  Ok(())
}

#[derive(Debug)]
struct GitInfo {
  commit_hash: String,
  commit_message: String,
  commit_author: String,
  commit_date: String,
}

fn get_tag_commits() -> Result<String> {
  use e_utils::cmd::Cmd;

  // 获取所有标签的提交历史
  let tag_commits = Cmd::new("git")
    .args([
      "log",
      "--tags",
      "--no-walk",
      "--date=\"format-local:%Y-%m-%d %H:%M:%S\"",
      "--format=\"commit: %h\n标签: %D\n时间: %cd\n作者: %an <%ae>\n说明: %s\n\"",
    ])
    .output()
    .map(|output| {
      let commits = output.stdout.trim().to_string();
      if commits.is_empty() {
        "无标签提交记录".to_string()
      } else {
        format!("\n{}", commits)
      }
    })
    .unwrap_or_else(|_| "获取失败".to_string());

  Ok(tag_commits)
}

fn get_git_info() -> GitInfo {
  use e_utils::cmd::Cmd;

  // 检查是否在 Git 仓库中
  let is_git_repo = Cmd::new("git").args(["rev-parse", "--is-inside-work-tree"]).output().is_ok();

  if !is_git_repo {
    return GitInfo {
      commit_hash: "未知".to_string(),
      commit_message: "非Git仓库".to_string(),
      commit_author: "未知".to_string(),
      commit_date: "未知".to_string(),
    };
  }

  let commit_hash = Cmd::new("git")
    .args(["rev-parse", "--short", "HEAD"])
    .output()
    .map(|v| v.stdout.to_string())
    .unwrap_or("获取失败".to_string());

  let commit_message = Cmd::new("git")
    .args(["log", "-1", "--pretty=%B"])
    .output()
    .map(|v| v.stdout.to_string())
    .unwrap_or("获取失败".to_string());

  let commit_author = Cmd::new("git")
    .args(["log", "-1", "--pretty=\"%an <%ae>\""])
    .output()
    .map(|output| output.stdout.to_string())
    .unwrap_or("获取失败".to_string());

  let commit_date = Cmd::new("git")
    .args(["log", "-1", "--pretty=%cd", "--date=format:\"%Y-%m-%d %H:%M:%S\""])
    .output()
    .map(|output| output.stdout.to_string())
    .unwrap_or("获取失败".to_string());

  GitInfo {
    commit_hash,
    commit_message,
    commit_author,
    commit_date,
  }
}
