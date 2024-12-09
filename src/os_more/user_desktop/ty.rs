use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// DesktopItems
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DesktopItem {
    /// 用户名
    pub uname: String,
    /// 路径
    pub path: PathBuf,
    /// 是否目录
    pub is_dir: bool,
    /// 是否隐藏
    pub is_hidden: bool,
    /// 属性ATTRIBUTE
    pub attribute: u32,
}