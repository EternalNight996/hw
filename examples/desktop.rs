#[tokio::main]
async fn main() -> e_utils::AnyResult<()> {
  #[cfg(feature = "desktop")]
  {
    use hw::os_more::{query_os_more, Type};
    hw::p(query_os_more(&[Type::Desktop], &["print"], &[], false).await?.join("\n"));
    hw::p(query_os_more(&[Type::Desktop], &["nodes"], &[], false).await?.join("\n"));
    hw::p("\nInfo Done\n")
  }
  Ok(())
}
