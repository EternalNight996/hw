mod ty;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod win;

pub use ty::*;

#[cfg(unix)]
pub use unix::*;
#[cfg(windows)]
pub use win::*;

#[cfg(test)]
mod tests {
  use super::*;

  // 平台无关的基础测试
  #[test]
  fn test_get_desktop_items_basic() {
    let items = get_desktop_items(None, None, &vec![]);
    // 至少应该能找到一些桌面项目
    assert!(!items.is_empty(), "应该能找到至少一些桌面项目");
  }

  #[test]
  fn test_filter_items() {
    // 测试文件名过滤
    let filters = vec!["desktop.ini"];
    let items = get_desktop_items(None, None, &filters);

    // 确保过滤的文件名不在结果中
    assert!(
      items
        .iter()
        .all(|item| !filters.contains(&item.path.file_name().and_then(|n| n.to_str()).unwrap_or(""))),
      "过滤的文件不应该出现在结果中"
    );
  }

  #[test]
  fn test_current_user_items() {
    if let Ok(username) = std::env::var("USER").or_else(|_| std::env::var("USERNAME")) {
      let items = get_desktop_items(Some(username.as_str()), None, &vec![]);

      // 确保找到的所有项目都属于当前用户
      for item in &items {
        assert_eq!(item.uname.to_lowercase(), username.to_lowercase(), "所有项目都应该属于指定用户");
      }
    }
  }

  // Windows 特定的测试
  #[cfg(windows)]
  mod windows_tests {
    use super::*;
    use std::os::windows::fs::MetadataExt;
    use windows::Win32::Storage::FileSystem::*;

    #[test]
    fn test_hidden_attributes() {
      let items = get_desktop_items(None, Some(FILE_ATTRIBUTE_HIDDEN), &vec![]);

      for item in &items {
        if let Ok(metadata) = std::fs::metadata(&item.path) {
          let attrs = metadata.file_attributes();
          assert!(attrs & FILE_ATTRIBUTE_HIDDEN != 0, "文件 {} 应该是隐藏的", item.path.display());
        }
      }
    }

    #[test]
    fn test_windows_system_users() {
      let users = get_system_users();
      assert!(!users.is_empty(), "应该能找到至少一个系统用户");

      if let Ok(current_user) = std::env::var("USERNAME") {
        assert!(users.iter().any(|u| u.eq_ignore_ascii_case(&current_user)), "当前用户应该在用户列表中");
      }
    }
  }

  // Unix 特定的测试
  #[cfg(unix)]
  mod unix_tests {
    use super::*;
    use std::os::unix::fs::MetadataExt;

    #[test]
    fn test_hidden_files() {
      let items = get_desktop_items(None, None, &vec![]);

      for item in &items {
        let is_hidden = item
          .path
          .file_name()
          .and_then(|n| n.to_str())
          .map(|s| s.starts_with('.'))
          .unwrap_or(false);

        assert_eq!(item.is_hidden, is_hidden, "隐藏状态应该与文件名是否以点开头一致: {}", item.path.display());
      }
    }

    #[test]
    fn test_unix_permissions() {
      let items = get_desktop_items(None, None, &vec![]);

      for item in &items {
        if let Ok(metadata) = std::fs::metadata(&item.path) {
          assert_eq!(item.attribute, metadata.mode(), "文件权限应该正确反映: {}", item.path.display());
        }
      }
    }

    #[test]
    fn test_unix_system_users() {
      let users = get_system_users();
      assert!(!users.is_empty(), "应该能找到至少一个系统用户");

      if let Ok(current_user) = std::env::var("USER") {
        assert!(users.iter().any(|u| u.eq_ignore_ascii_case(&current_user)), "当前用户应该在用户列表中");
      }
    }
  }
}
