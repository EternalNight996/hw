#[tokio::main]
async fn main() -> e_utils::AnyResult<()> {
  #[cfg(feature = "disk")]
  {
    use hw::disk::disk_query;
    // hw::p(disk_query::<&str>("data", &[], &[]).await?);
    // hw::p(disk_query::<&str>("mount-tree", &[], &["C:"]).await?);
     // hw::p(disk_query::<&str>("check-load", &["10", "90"], &[]).await?);
    hw::p(disk_query::<&str>("info", &[], &[]).await?);
  }
  Ok(())
}
