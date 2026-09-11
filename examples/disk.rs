#[tokio::main]
async fn main() -> e_utils::AnyResult<()> {
  #[cfg(feature = "disk")]
  {
    use hw::disk::disk_query;
    // hw::p(disk_query::<&str>("data", &[], &[], false).await?);
    // hw::p(disk_query::<&str>("mount-tree", &[], &["C:"], false).await?);
    // hw::p(disk_query::<&str>("check-load", &["10", "90"], &[], false).await?);
    hw::p(disk_query::<&str>("info", &[], &[], false).await?);
  }
  Ok(())
}
