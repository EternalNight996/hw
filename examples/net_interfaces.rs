use hw::os_more::{query_os_more, Type};
#[tokio::main]
async fn main() -> e_utils::AnyResult<()> {
  #[cfg(feature = "network")]
  {
    // -- 条件
    // "~Less100" 速度小于100
    // "~100" 速度大于等于100
    // "~1000" 速度大于等于1000
    // "~Big1000" 速度大于等于10000
    // "~is_connected" 正在连接
    // "~has_dhcp_ip" 有DHCP IP
    hw::p(query_os_more(&[Type::NetInterface], &["print"], &["~Less100","~is_connected"], false).await?.len().to_string());
    hw::p(format!("{:#?}", query_os_more(&[Type::NetInterface], &["nodes"], &["~Less100"], false).await?.join("\n")));
    hw::p(query_os_more(&[Type::NetInterface], &["nodes"], &["~has_dhcp_ip"], false).await?.len().to_string());
    hw::p(query_os_more(&[Type::NetInterface], &["old"], &["~is_connected"], false).await?.len().to_string());
    hw::p(query_os_more(&[Type::NetInterface], &["check-mac","*I225-V #1"], &["~has_dhcp_ip"], true).await?.join("\n"));
  }
  Ok(())
}

