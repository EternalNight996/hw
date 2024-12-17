use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

/// 驱动状态类型
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, Display, EnumString)]
pub enum DriveStatusType {
  Runing,
  Disabled,
  Stopped,
  Error,
  Hidden,
  #[default]
  None,
}

/// # 驱动信息
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct DriveInfo {
  pub id: String,
  pub driver_descript: String,
  pub status: DriveStatusType,
}

/// # 驱动Node信息
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct DriveNodeInfo {
  pub id: String,
  pub drive_node: String,
  pub driver_descript: String,
  pub status: DriveStatusType,
  pub name: String,
  pub inf_file: String,
  pub inf_section: String,
  pub manufacturer_name: String,
  pub provider_name: String,
  pub driver_date: String,
  pub driver_version: String,
  pub driver_node_rank: String,
  pub driver_node_flags: String,
  pub signed: bool,
}

impl From<DriveInfo> for DriveNodeInfo {
  fn from(value: DriveInfo) -> Self {
    let mut dninfo = DriveNodeInfo::default();
    dninfo.id = value.id;
    dninfo.driver_descript = value.driver_descript;
    dninfo.status = value.status;
    dninfo
  }
}
