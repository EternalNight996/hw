use hw::os_more::{query_os_more, Type};
#[tokio::main]
async fn main() -> e_utils::AnyResult<()> {
  #[cfg(feature = "network")]
  {
    // "~Less100" => x.speed() < 100,
    // "~100" => x.speed() >= 100,
    // "~1000" => x.speed() >= 1000,
    // "~Big1000" => x.speed() >= 10000,
    // "~is_connected" => x.is_connected(),
    // "~has_dhcp_ip" => x.has_dhcp_ip(),
    println!("{}", query_os_more(&[Type::NetInterface], &["print"], &["~Less100","~is_connected"], false).await?.len());
    println!("{:#?}", query_os_more(&[Type::NetInterface], &["nodes"], &["~Less100"], false).await?.join("\n"));
    println!("{}", query_os_more(&[Type::NetInterface], &["nodes"], &["~has_dhcp_ip"], false).await?.len());
    println!("{}", query_os_more(&[Type::NetInterface], &["old"], &["~is_connected"], false).await?.len());
    println!("{}", query_os_more(&[Type::NetInterface], &["check-mac","*I225-V #1"], &["~has_dhcp_ip"], true).await?.join("\n"));
  }
  Ok(())
}

