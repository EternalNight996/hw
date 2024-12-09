use hw::os_more::{query_os_more, Type};
#[tokio::main]
async fn main() -> e_utils::AnyResult<()> {
  #[cfg(feature = "system")]
  {
    println!("{}", query_os_more(&[Type::NetInterface], ["print"], [], false).await?.len());
    println!("{}", query_os_more(&[Type::NetInterface], ["nodes"], [], false).await?.len());
    println!("{}", query_os_more(&[Type::NetInterface], ["nodes"], ["=net"], false).await?.len());
    println!("{}", query_os_more(&[Type::NetInterface], ["old"], [], false).await?.len());
    println!("{}", query_os_more(&[Type::NetInterface], ["check-mac","*I225-V #1"], ["=net"], false).await?.join("\n"));
    println!("\nInfo Done\n")
  }
  Ok(())
}
