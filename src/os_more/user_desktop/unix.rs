use std::path::PathBuf;
use super::types::DesktopItem;

pub fn get_desktop_items(
    query_user: Option<String>,
    _attr_filter: Option<u32>,
    filters: &Vec<String>,
) -> Vec<DesktopItem> {
    let mut items = vec![];
    let mut users = get_system_users();
    
    if let Some(query) = query_user {
        if users.iter().any(|v| v == &query) {
            users = vec![query];
        }
    }

    for uname in users {
        let desktop_path = PathBuf::from("/home").join(&uname).join("Desktop");
        
        if desktop_path.exists() {
            process_desktop_entries(&desktop_path, &uname, filters, &mut items);
        }
    }

    items
}

fn process_desktop_entries(
    desktop_path: &PathBuf,
    uname: &str,
    filters: &Vec<String>,
    items: &mut Vec<DesktopItem>,
) {
    if let Ok(entries) = std::fs::read_dir(desktop_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            
            if entry
                .file_name()
                .into_string()
                .map(|fname| filters.contains(&fname))
                .unwrap_or_default()
            {
                continue;
            }

            let is_hidden = entry
                .file_name()
                .to_str()
                .map(|s| s.starts_with('.'))
                .unwrap_or(false);

            let attribute = entry
                .metadata()
                .map(|meta| meta.mode())
                .unwrap_or(0);

            items.push(DesktopItem {
                uname: uname.to_string(),
                is_dir: path.is_dir(),
                is_hidden,
                path,
                attribute,
            });
        }
    }
}

pub fn get_system_users() -> Vec<String> {
    let mut users = Vec::new();
    
    if let Ok(entries) = std::fs::read_dir("/home") {
        for entry in entries.flatten() {
            if let Ok(file_name) = entry.file_name().into_string() {
                users.push(file_name);
            }
        }
    }
    
    users
}