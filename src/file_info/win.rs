use std::ffi::OsStr;
  use std::os::windows::ffi::OsStrExt;
  use std::path::PathBuf;
  use winapi::shared::minwindef::DWORD;
  use winapi::um::processenv::SearchPathW;

  /// 寻找完整路径
  pub fn find_dll_path(dll_name: &str) -> Option<std::path::PathBuf> {
    let wide_dll_name: Vec<u16> = OsStr::new(dll_name)
      .encode_wide()
      .chain(std::iter::once(0))
      .collect();

    let mut buffer = [0u16; 260]; // MAX_PATH
    let mut copied: DWORD = 0;

    unsafe {
      let result = SearchPathW(
        std::ptr::null(),
        wide_dll_name.as_ptr(),
        std::ptr::null(),
        buffer.len() as DWORD,
        buffer.as_mut_ptr(),
        std::ptr::null_mut(),
      );

      if result > 0 {
        copied = result;
      }
    }

    if copied > 0 {
      let path = String::from_utf16_lossy(&buffer[..copied as usize]);
      Some(PathBuf::from(path))
    } else {
      None
    }
  }