use hw::os_more::{query_os_more, Type};
#[tokio::main]
async fn main() -> e_utils::AnyResult<()> {
  #[cfg(feature = "network")]
  {
    hw::p(query_os_more(&[Type::NetManage], &["ping", "127.0.0.1", "baidu.com", "3"], &[], false)
        .await?
        .join("\n"));
    hw::p(query_os_more(&[Type::NetManage], &["ping-nodes", "baidu.com", "3"], &[], false)
          .await?
          .join("\n"));
  }
  Ok(())
}
