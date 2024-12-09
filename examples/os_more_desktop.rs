use hw::os_more::{query_os_more, Type};
#[tokio::main]
async fn main() -> e_utils::AnyResult<()> {
  #[cfg(feature = "system")]
  {
    println!("{}", query_os_more(&[Type::Desktop], ["print"], [], false).await?.join("\n"));
    println!("{}", query_os_more(&[Type::Desktop], ["nodes"], [], false).await?.join("\n"));
    println!("\nInfo Done\n")
  }
  Ok(())
}
