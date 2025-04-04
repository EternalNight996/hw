pub mod ty;
pub use ty::*;
#[cfg(all(target_os = "windows", feature = "drive"))]
pub mod win;
#[cfg(all(target_os = "windows", feature = "drive"))]
pub use win::*;

#[allow(unused)]
pub async fn drive_query<T: AsRef<str>>(
  task: &str,
  args: impl IntoIterator<Item = T>,
  filter: impl IntoIterator<Item = T>,
  is_full: bool,
) -> e_utils::AnyResult<String> {
  #[cfg(not(all(target_os = "windows", feature = "drive")))]
  return Err("Not Windows".into());
  #[cfg(all(target_os = "windows", feature = "drive"))]
  {
    let args: Vec<String> = args.into_iter().map(|x| x.as_ref().to_string()).collect();
    let filter: Vec<String> = filter.into_iter().map(|x| x.as_ref().to_string()).collect();
    return match task {
      "check-status" => {
        let _ = pnputil_scan()?;
        let list = crate::drive::findnodes_status(&filter)?;
        let mut err = 0;
        let mut ok = 0;
        for node in list.into_iter() {
          if node.status == DriveStatusType::Error || node.status == DriveStatusType::None || node.status == DriveStatusType::Disabled {
            err += 1;
            if is_full {
              crate::ep(format!("{err}. Err: {:#?}\n", crate::drive::findnodes_full(vec![node], &filter)?));
            } else {
              crate::ep(format!("{err}. Err: {:#?}\n", node));
            }
          } else {
            ok += 1;
          }
        }
        let msg = format!("PASS: {ok}; FAIL: {err}");
        if err > 0 {
          Err(msg.into())
        } else {
          Ok(msg)
        }
      }
      "nodes-status" => {
        let list = crate::drive::findnodes_status(&filter)?;
        if is_full {
          let list = crate::drive::findnodes_full(list, &filter)?;
          Ok(serde_json::to_string(&list)?)
        } else {
          Ok(serde_json::to_string(&list)?)
        }
      }
      "print-status" => {
        let list = crate::drive::findnodes_status(&filter)?;
        let count = list.len();
        if is_full {
          let list = crate::drive::findnodes_full(list, &filter)?;
          for node in &list {
            crate::p(serde_json::to_string_pretty(&node)?);
          }
        } else {
          for node in &list {
            crate::p(serde_json::to_string_pretty(&node)?);
          }
        }
        Ok(format!("COUNT: {count}"))
      }
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
        let list = crate::drive::findnodes_full(crate::drive::findnodes(&filter)?, &filter)?;
        let mut nodes = vec![];
        for node in &list {
          let inf_path: std::path::PathBuf = (&node.inf_file).into();
          let fname = inf_path.file_name().and_then(|v| v.to_str()).ok_or("File name error")?;
          let mut new_args = args.clone();
          new_args.insert(0, fname.to_string());
          let _dres = crate::drive::pnputil_delete_driver(new_args)?;
          let _ = crate::drive::pnputil_scan()?;
          let status = crate::drive::findnodes_status(&vec![node.id.clone()])?
            .get(0)
            .cloned()
            .ok_or("Get Status Error")?;
          if status.status != DriveStatusType::None {
            return Err("Delete Error: not null".into());
          }
          nodes.push(node)
        }
        Ok(serde_json::to_string(&nodes)?)
      }
      "print" => {
        let list = crate::drive::findnodes(&filter)?;
        let count = list.len();
        if is_full {
          let list = crate::drive::findnodes_full(list, &filter)?;
          for node in &list {
            crate::p(serde_json::to_string_pretty(&node)?);
          }
        } else {
          for node in &list {
            crate::p(serde_json::to_string_pretty(&node)?);
          }
        }
        Ok(format!("COUNT: {count}"))
      }
      "nodes" => {
        let list = crate::drive::findnodes(&filter)?;
        if is_full {
          let list = crate::drive::findnodes_full(list, &filter)?;
          Ok(serde_json::to_string(&list)?)
        } else {
          Ok(serde_json::to_string(&list)?)
        }
      }
      "restart" => {
        let status_list = crate::drive::find_with_run(&args, &filter, crate::drive::pnputil_restart)?;
        for status in &status_list {
          if status.status == DriveStatusType::Runing {
            break;
          }
        }
        Ok(serde_json::to_string(&status_list)?)
      }
      "enable" => {
        let status_list = crate::drive::find_with_run(&args, &filter, crate::drive::pnputil_enable)?;
        for status in &status_list {
          if status.status != DriveStatusType::Runing {
            return Err("Enable Error: not runing".into());
          }
        }
        Ok(serde_json::to_string(&status_list)?)
      }
      "disable" => {
        let status_list = crate::drive::find_with_run(&args, &filter, crate::drive::pnputil_disable)?;
        for status in &status_list {
          if status.status != DriveStatusType::Disabled {
            return Err("Disable Error: not disabled".into());
          }
        }
        Ok(serde_json::to_string(&status_list)?)
      }
      "remove" => {
        let status_list = crate::drive::find_with_run(&args, &filter, crate::drive::pnputil_remove)?;
        for status in &status_list {
          if status.status != DriveStatusType::None {
            return Err("Remove Error: not none".into());
          }
        }
        Ok(serde_json::to_string(&status_list)?)
      }
      "export" => Ok(crate::drive::pnputil_export_driver(args.clone())?),
      _ => crate::drive::devcon(args),
    };
  }
}
