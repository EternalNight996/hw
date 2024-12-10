#[tokio::main]
async fn main() -> e_utils::AnyResult<()> {
  #[cfg(feature = "network")]
  {
    hw::p(hw::os_more::query_os_more(&[hw::os_more::Type::NetManage], &["sync-datetime", "time.windows.com"], &[], false)
        .await?
        .join("\n"));
  }
  Ok(())
}
