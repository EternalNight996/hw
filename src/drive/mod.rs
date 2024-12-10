pub mod ty;
pub use ty::*;
#[cfg(target_os = "windows")]
pub mod win;
#[cfg(target_os = "windows")]
pub use win::*;

pub async fn drive_query<T: AsRef<str>>(
  task: &str,
  args: impl IntoIterator<Item = T>,
  filter: impl IntoIterator<Item = T>,
  is_full: bool,
) -> e_utils::AnyResult<String> {
  #[cfg(not(target_os = "windows"))]
  return Err("Not Windows".into());
  #[cfg(target_os = "windows")]
  {
    let args: Vec<String> = args.into_iter().map(|x| x.as_ref().to_string()).collect();
    let filter: Vec<String> = filter.into_iter().map(|x| x.as_ref().to_string()).collect();
    return match task {
      "scan" => pnputil_scan(),
      "add-folder" => {
        let mut outres = String::new();
        let target = args.get(0).ok_or("Args Error Target 1 ")?;
        for x in e_utils::fs::tree_folder(&target)?
          .into_iter()
          .filter(|x| std::path::Path::new(x).extension().unwrap_or_default() == "inf")
        {
          let mut new_args = args.clone();
          if let Some(v) = new_args.get_mut(0) {
            *v = x.to_string_lossy().to_string();
          }
          let res = pnputil_add_driver(new_args)?;
          outres += &format!("{:?};;", res);
        }
        Ok(outres)
      }
      "add" => return pnputil_add_driver(args),
      "delete" => return pnputil_delete_driver(args),
      "delete-find" => {
        let list = crate::drive::findnodes(&filter, is_full)?;
        let mut nodes = vec![];
        for node in &list {
          let inf_path: std::path::PathBuf = (&node.inf_file).into();
          let fname = inf_path.file_name().and_then(|v| v.to_str()).ok_or("File name error")?;
          let mut new_args = args.clone();
          new_args.insert(0, fname.to_string());
          let _dres = crate::drive::pnputil_delete_driver(new_args)?;
          let status = crate::drive::devcon_status(&node.id)?;
          if status.status != DriveStatusType::None {
            return Err("Delete Error: not null".into());
          }
          nodes.push(node)
        }
        Ok(serde_json::to_string_pretty(&nodes)?)
      }
      "print" => {
        let list = crate::drive::findnodes(&filter, is_full)?;
        let count = list.len();
        for node in &list {
          crate::p(serde_json::to_string_pretty(&node)?);
        }
        Ok(format!("COUNT: {count}"))
      }
      "nodes" => {
        let list = crate::drive::findnodes(&filter, is_full)?;
        Ok(serde_json::to_string_pretty(&list)?)
      }
      "restart" => {
        let status_list = crate::drive::find_with_run(&args, &filter, is_full, crate::drive::pnputil_restart)?;
        for status in &status_list {
          if status.status == DriveStatusType::Runing {
            break;
          }
        }
        Ok(serde_json::to_string_pretty(&status_list)?)
      }
      "enable" => {
        let status_list = crate::drive::find_with_run(&args, &filter, is_full, crate::drive::pnputil_enable)?;
        for status in &status_list {
          if status.status != DriveStatusType::Runing {
            return Err("Enable Error: not runing".into());
          }
        }
        Ok(serde_json::to_string_pretty(&status_list)?)
      }
      "disable" => {
        let status_list = crate::drive::find_with_run(&args, &filter, is_full, crate::drive::pnputil_disable)?;
        for status in &status_list {
          if status.status != DriveStatusType::Disabled {
            return Err("Disable Error: not disabled".into());
          }
        }
        Ok(serde_json::to_string_pretty(&status_list)?)
      }
      "remove" => {
        let status_list = crate::drive::find_with_run(&args, &filter, is_full, crate::drive::pnputil_remove)?;
        for status in &status_list {
          if status.status != DriveStatusType::None {
            return Err("Remove Error: not none".into());
          }
        }
        Ok(serde_json::to_string_pretty(&status_list)?)
      }
      "export" => Ok(crate::drive::pnputil_export_driver(args.clone())?),
      _ => crate::drive::devcon(args),
    };
  }
}
