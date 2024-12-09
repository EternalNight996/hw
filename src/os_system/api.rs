use std::{fs, io::Write as _, path::Path};

use e_utils::{cmd::Cmd, parse::MyParseFormat as _, regex::Regex};

use super::ActiveLocalType;

pub async fn os_system_query(task: &str, args: &Vec<String>, _filter: &Vec<String>, _is_full: bool) -> e_utils::AnyResult<String> {
  #[cfg(target_os = "windows")]
  return match task {
    "check-with-cache" => {
      let query = args.get(0).ok_or("Args Error must > 0 ")?.clone();
      let res = check_os_active().await?;
      let code = ActiveLocalType::Temp(query).query_cache().await?;
      Ok(format!("{};{}", res, code))
    }
    "check" => check_os_active().await,
    "active" => {
      let code = args.get(0).ok_or("Args Error must > 1 ")?;
      let tmp_fname = args.get(1).cloned().unwrap_or("hw_os_active".to_string());
      let temp_type = ActiveLocalType::Temp(tmp_fname);
      active_os(code, temp_type).await
    }
    "deactive" => deactivate_os().await,
    "rkms" => {
      let v = args.get(0).ok_or("Args Error must > 0 ")?;
      register_kms(v).await
    }
    "ckms" => clear_kms().await,
    "clean-cache" => {
      let args = args.get(0).ok_or("Args Error must > 0 ")?.clone();
      Ok(ActiveLocalType::Temp(args).clean_cache()?)
    }
    "query-cache" => {
      let args = args.get(0).ok_or("Args Error must > 0 ")?.clone();
      ActiveLocalType::Temp(args).query_cache().await
    }
    _ => Err("Task Error".into()),
  };
  #[cfg(not(feature = "windows"))]
  Err("OS System not supported".into())
}

/// # 检查OS是否激活
/// # Example sh
/// ```sh
/// e-app.exe --api active --task check
/// ```
pub async fn check_os_active() -> e_utils::AnyResult<String> {
  // 执行 slmgr.vbs 命令
  let output = Cmd::new("cscript")
    .args(["/nologo", "C:\\Windows\\System32\\slmgr.vbs", "-xpr"])
    .a_output()
    .await?;
  let output = output.stdout;
  // 在输出中查找激活状态
  if output.contains("激活") || output.contains("activated") {
    return Ok(output);
  }
  Err(output.into())
}

/// 激活系统
/// # Example sh
/// ```sh
/// e-app.exe --api active --task active -- YVWGF-BXNMC-HTQYQ-CPQ99-66QFC fname.txt
/// ```
pub async fn active_os(product_key: &str, save_type: ActiveLocalType) -> e_utils::AnyResult<String> {
  if let Ok(re) = Regex::new(r"^[0-9A-Z]{5}-(?:[0-9A-Z]{5}-){3}[0-9A-Z]{5}$") {
    // Define the optimized regular expression pattern for a Microsoft product key
    if !re.is_match(product_key) {
      return Err(format!("Error: Active Code of Rule,Please check;{product_key}").into());
    }
  }
  let cmd = Cmd::new("cscript").args(["/nologo", "C:\\Windows\\System32\\slmgr.vbs"]);
  // Execute the slmgr.vbs script with the /ipk argument to install the product key
  let output = cmd
    .clone()
    .args(["/ipk", product_key])
    .a_output()
    .await
    .map_err(|e| format!("Error: Install Product Active Code: {product_key};{e}"))?;
  // Activate Windows using the installed product key
  let o2 = cmd.arg("/ato").output().map_err(|e| format!("Error: Active Code: {product_key};{e}"))?;
  // 在输出中查找激活状态
  if o2.stdout.contains("激活") || o2.stdout.contains("activated") {
    match save_type {
      ActiveLocalType::Temp(fname) => {
        if let Ok(tmp) = "%TEMP%".parse_env() {
          let path = Path::new(&tmp).join(&format!("os-key-{fname}"));
          if let Ok(mut f) = fs::OpenOptions::new().read(true).write(true).create(true).open(&path) {
            let _ = f.write(product_key.as_bytes());
          }
        }
      }
    }
    return Ok(format!("{};{}", output.stdout, o2.stdout));
  }
  Err(format!("Error: Active Code: {product_key}; {}", o2.stdout).into())
}

/// # 取消注册
/// # Example sh
/// ```sh
/// e-app.exe --api active --task deactive
/// ```
pub async fn deactivate_os() -> e_utils::AnyResult<String> {
  let cmd = Cmd::new("cscript").args(["/nologo", "C:\\Windows\\System32\\slmgr.vbs"]);
  let out = cmd.output().map_err(|e| format!("Error: Uninstall the product key;{e}"))?;
  let x2 = cmd
    .clone()
    .arg("/cpky")
    .a_output()
    .await
    .map_err(|e| format!("Error: remove the product key from the registry;{e}"))?;
  let x3 = cmd
    .arg("/rearm")
    .a_output()
    .await
    .map_err(|e| format!("Error: remove the product key from the registry;{e}"))?;
  Ok(format!("{};{};{}", out.stdout, x2.stdout, x3.stdout))
}

/// # 注册KMS
/// # Example sh
/// ```sh
/// e-app.exe --api active --task rkms -- kms.03k.org
/// ```
pub async fn register_kms(server: &str) -> e_utils::AnyResult<String> {
  Ok(
    Cmd::new("cscript")
      .args(["/nologo", "C:\\Windows\\System32\\slmgr.vbs", "/skms"])
      .a_output()
      .await
      .map_err(|e| format!("Error: registry KMS {server};{e}"))?
      .stdout,
  )
}

/// # 清除注册KMS
/// # Example sh
/// ```sh
/// e-app.exe --api active --task ckms
/// ```
pub async fn clear_kms() -> e_utils::AnyResult<String> {
  Ok(
    Cmd::new("cscript")
      .args(["/nologo", "C:\\Windows\\System32\\slmgr.vbs", "/ckms"])
      .a_output()
      .await
      .map_err(|e| format!("Error: Clear KMS;{e}"))?
      .stdout,
  )
}
