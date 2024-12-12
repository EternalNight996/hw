#[allow(unused)]
pub async fn file_info_query<T: AsRef<str>>(task: &str, args: impl IntoIterator<Item = T>) -> e_utils::AnyResult<String> {
  #[cfg(not(all(feature = "file-info", target_os = "windows")))]
  return Err("Not Support".into());
  #[cfg(all(feature = "file-info", target_os = "windows"))]
  {
    let args: Vec<String> = args.into_iter().map(|x| x.as_ref().to_string()).collect();
    let src = args.get(0).ok_or("Args Error must > 0 ")?;
    use e_utils::fs::AutoPath;
    match &*task {
      "copy-lib" => {
        let to = args.get(1).ok_or("Args Error must > 1 ")?;
        to.auto_create_dir()?;
        Ok(crate::file_info::lib_copy(src, to).map(|v| format!("Copy count {}", v))?)
      }
      "print" => {
        let res = serde_json::to_string(&crate::file_info::a_open(src).await?)?;
        crate::p(&res);
        return Ok(res);
      }
      "nodes" => Ok(serde_json::to_string(&crate::file_info::a_open(src).await?)?),
      _ => return Err("Task Error".into()),
    }
  }
}
