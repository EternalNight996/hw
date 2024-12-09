use std::os::windows::fs::MetadataExt;
use std::path::PathBuf;
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::NetworkManagement::NetManagement::{
    NetApiBufferFree, NetUserEnum, USER_INFO_1,
};
use windows::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_HIDDEN, INVALID_FILE_ATTRIBUTES};

use super::ty::DesktopItem;

pub fn get_desktop_items(
    query_user: Option<String>,
    attr_filter: Option<u32>,
    filters: &Vec<String>,
) -> Vec<DesktopItem> {
    let mut items = vec![];
    let mut users = get_system_users();
    users.push("Public".to_string());
    
    if let Some(query) = query_user {
        if users.iter().any(|v| v == &query) {
            users = vec![query];
        }
    }

    if let Some(system_drive) = std::env::var_os("SystemDrive") {
        for uname in users {
            let desktop_path = PathBuf::from(&system_drive)
                .join("Users")
                .join(&uname)
                .join("Desktop");

            if desktop_path.exists() {
                process_desktop_entries(&desktop_path, &uname, attr_filter, filters, &mut items);
            }
        }
    }

    items
}

fn process_desktop_entries(
    desktop_path: &PathBuf,
    uname: &str,
    attr_filter: Option<u32>,
    filters: &Vec<String>,
    items: &mut Vec<DesktopItem>,
) {
    if let Ok(entries) = std::fs::read_dir(desktop_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            let attr = if let Ok(meta) = entry.metadata() {
                meta.file_attributes()
            } else {
                INVALID_FILE_ATTRIBUTES
            };

            if let Some(filter) = attr_filter {
                if attr == INVALID_FILE_ATTRIBUTES || (attr & filter) != filter {
                    continue;
                }
            }

            if entry
                .file_name()
                .into_string()
                .map(|fname| filters.contains(&fname))
                .unwrap_or_default()
            {
                continue;
            }

            items.push(DesktopItem {
                uname: uname.to_string(),
                is_dir: path.is_dir(),
                is_hidden: attr & FILE_ATTRIBUTE_HIDDEN != 0,
                path,
                attribute: attr,
            });
        }
    }
}

pub fn get_system_users() -> Vec<String> {
    let mut users = Vec::new();

    unsafe {
        let mut buf = std::ptr::null_mut();
        let mut entries_read = 0u32;
        let mut total_entries = 0u32;
        let mut resume_handle = 0u32;

        let result = NetUserEnum(
            None,
            1,
            0,
            &mut buf,
            u32::MAX,
            &mut entries_read,
            &mut total_entries,
            &mut resume_handle,
        );

        if result == ERROR_SUCCESS {
            let user_info = std::slice::from_raw_parts(buf as *const USER_INFO_1, entries_read as usize);
            for info in user_info {
                if let Ok(name) = String::from_utf16(std::slice::from_raw_parts(
                    info.usri1_name.0 as *const u16,
                    wcslen(info.usri1_name.0),
                )) {
                    users.push(name);
                }
            }
            NetApiBufferFree(buf as *mut ::core::ffi::c_void);
        }
    }

    users
}

unsafe fn wcslen(ptr: *const u16) -> usize {
    let mut len = 0;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    len
}