#[tokio::main]
async fn main() -> e_utils::AnyResult<()> {
  #[cfg(feature = "system")]
  {
    use hw::os_more::{query_os_more, Type};
    let empty: Vec<String> = vec![];
    hw::p(query_os_more(&[Type::MemoryManufacturerPartNumber], &empty, &[], false).await?.join("\n"));
    // you also can take api; like memory total
    hw::p("\nMemory Info Done\n")
  }
  Ok(())
}
