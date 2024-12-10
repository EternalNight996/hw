use hw::os_more::{query_os_more, Type};
#[tokio::main]
async fn main() -> e_utils::AnyResult<()> {
  #[cfg(feature = "network")]
  {
    hw::p(query_os_more(&[Type::NetManage], &["dhcp"], &["~has_dhcp_ip"], false).await?.join("\n"));
  }
  Ok(())
}
