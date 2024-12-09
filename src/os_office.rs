use e_utils::{cmd::Cmd, regex::Regex};
use std::{
  path::{Path, PathBuf},
  str::FromStr as _,
};
use strum::*;

use crate::os_system::ActiveLocalType;

pub async fn os_office_query(task: &str, args: &Vec<String>, _filter: &Vec<String>, _is_full: bool) -> e_utils::AnyResult<String> {
  #[cfg(target_os = "windows")]
  {
    let v1 = args.get(0).ok_or("Args Error must > 0 ")?;
    let version = OfficeVersion::from_str(v1)?;
    return match task {
      "check" => check_office(version).await,
      "rkms" => register_office_kms(version, args.get(1).ok_or("Args Error must > 1 -> KMS Server")?).await,
      "active" => active_office(version, args.get(1).ok_or("Args Error must > 1 -> Active Code")?).await,
      "check-with-cache" => {
        let fname = format!("office-{}", args.get(0).ok_or("Args Error must > 0 -> Active Code")?.to_string());
        let code = ActiveLocalType::Temp(fname).query_cache().await?;
        let res = check_office(version).await?;
        Ok(format!("{};{}", res, code))
      }
      "clean-cache" => {
        let fname = format!("office-{}", args.get(0).ok_or("Args Error must > 0 -> Active Code")?.to_string());
        Ok(ActiveLocalType::Temp(fname).clean_cache()?)
      }
      "query-cache" => {
        let fname = format!("office-{}", args.get(0).ok_or("Args Error must > 0 -> Active Code")?.to_string());
        ActiveLocalType::Temp(fname).query_cache().await
      }
      _ => Err("Task Error".into()),
    };
  }
  #[cfg(not(feature = "windows"))]
  Err("OS System not supported".into())
}

/// Office激活版本
#[derive(PartialEq, Clone, Debug, Display, EnumString)]
pub enum OfficeVersion {
  V2003,
  V2006,
  V2010,
  V2013,
  V2016,
  V2019,
  V365,
  None,
}
impl OfficeVersion {
  ///
  pub fn find_version(version: OfficeVersion, l: &Vec<(Self, PathBuf)>) -> (Self, PathBuf) {
    for x in l {
      if x.0 == version {
        return x.clone();
      }
    }
    (Self::None, PathBuf::new())
  }
  /// 获取凭证路径
  pub fn license_path(&self) -> Option<PathBuf> {
    Some(
      Path::new(match self {
        OfficeVersion::V2003 => "C:\\Program Files\\Microsoft Office\\root\\Licenses3",
        OfficeVersion::V2006 => "C:\\Program Files\\Microsoft Office\\root\\Licenses6",
        OfficeVersion::V2010 => "C:\\Program Files\\Microsoft Office\\root\\Licenses10",
        OfficeVersion::V2013 => "C:\\Program Files\\Microsoft Office\\root\\Licenses13",
        OfficeVersion::V2016 => "C:\\Program Files\\Microsoft Office\\root\\Licenses16",
        OfficeVersion::V2019 => "C:\\Program Files\\Microsoft Office\\root\\Licenses19",
        OfficeVersion::V365 => "C:\\Program Files\\Microsoft Office\\root\\Licenses365",
        OfficeVersion::None => return None,
      })
      .to_path_buf(),
    )
  }
}

/// # 检查OFFICE
/// # Example sh
/// ```sh
/// e-app.exe --api office --task check -- V2016
/// ```
pub async fn check_office(v: OfficeVersion) -> e_utils::AnyResult<String> {
  let olist = check_office_dir().unwrap_or_default();
  let office = OfficeVersion::find_version(v, &olist);
  match office.0 {
    OfficeVersion::None => Err(format!("Error: Check Office Not Found {}", office.1.display()).into()),
    _ => {
      let exe = "ospp.vbs";
      let exe_path = office.1.join(exe);
      if exe_path.exists() {
        Ok(
          Cmd::new("cscript")
            .args(["/nologo", &exe_path.to_string_lossy(), "/dstatus"])
            .cwd(office.1)
            .a_output()
            .await
            .map_err(|e| format!("Error: Office check;{e}"))?
            .stdout,
        )
      } else {
        Err(format!("Error: Check Office Not Found {}", exe_path.display()).into())
      }
    }
  }
}
/// # 注册OFFICE KMS
/// # Example sh
/// ```sh
/// e-app.exe --api office --task rkms -- V2016 kms.03k.org
/// ```
pub async fn register_office_kms(v: OfficeVersion, server: &str) -> e_utils::AnyResult<String> {
  let olist = check_office_dir().unwrap_or_default();
  let office = OfficeVersion::find_version(v, &olist);
  match office.0 {
    OfficeVersion::None => Err(format!("Error: Check Office Not Found").into()),
    _ => {
      let exe = "ospp.vbs";
      let exe_path = office.1.join(exe);
      if exe_path.exists() {
        Ok(
          Cmd::new("cscript")
            .args(["/nologo", &exe_path.to_string_lossy(), &format!("/sethst:{server}")])
            .cwd(office.1)
            .a_output()
            .await
            .map_err(|e| format!("Error: Office set KMS: {server};{e}"))?
            .stdout,
        )
      } else {
        Err(format!("Error: Check Office Not Found {}", exe_path.display()).into())
      }
    }
  }
}
/// # 激活OFFICE
/// # Example sh
/// ```sh
/// e-app.exe --api office --task active -- V2016 NMMKJ-6RK4F-KMJVX-8D9MJ-6MWKP
/// ```
pub async fn active_office(v: OfficeVersion, code: &str) -> e_utils::AnyResult<String> {
  let olist = check_office_dir().unwrap_or_default();
  let office = OfficeVersion::find_version(v, &olist);
  match office.0 {
    OfficeVersion::None => Err(format!("Error: Check Office Not Found").into()),
    _ => {
      let exe = "ospp.vbs";
      let exe_path = office.1.join(exe);
      if exe_path.exists() {
        let x2 = Cmd::new("cscript")
          .args(["/nologo", &exe_path.to_string_lossy(), &format!("/act"), code])
          .cwd(office.1)
          .a_output()
          .await
          .map_err(|e| format!("Error: Active Office;{e}"))?;
        if Regex::new("(successful|成功)")?.is_match(&x2.stdout) {
          Ok(x2.stdout)
        } else {
          Err(format!("Error: Active Office;{}", x2.stdout).into())
        }
      } else {
        Err(format!("Error: Check Office Not Found {}", exe_path.display()).into())
      }
    }
  }
}
/// 检查OFFICE路径
fn check_office_dir() -> Option<Vec<(OfficeVersion, PathBuf)>> {
  let p = Path::new("C:\\Program Files\\Microsoft Office");
  let mut l = vec![];
  if p.exists() {
    for x in p.read_dir().ok()? {
      if let Ok(dir) = x {
        let s = &*dir.file_name().to_string_lossy().to_string();
        let x = match s {
          "Office2003" => OfficeVersion::V2003,
          "Office2006" => OfficeVersion::V2006,
          "Office2010" => OfficeVersion::V2010,
          "Office2013" => OfficeVersion::V2013,
          "Office2016" => OfficeVersion::V2016,
          "Office2019" => OfficeVersion::V2019,
          "Office365" => OfficeVersion::V365,
          _ => OfficeVersion::None,
        };
        if x != OfficeVersion::None {
          l.push((x, dir.path()));
        }
      }
    }
  }
  Some(l)
}
