#[tokio::main]
async fn main() -> e_utils::AnyResult<()> {
  #[cfg(feature = "network")]
  {
    use hw::os_more::{query_os_more, Type};
    hw::p(query_os_more(
        &[Type::NetManage],
        &["set-ip", "以太网", "192.168.1.100", "255.255.255.0", "192.168.1.1"],
        &[],
        false
      )
      .await?
      .join("\n")
    );
    hw::p(query_os_more(
          &[Type::NetManage],
          &["set-dns", "以太网", "223.5.5.5", "114.114.114.114"],
          &[],
          false
        )
        .await?
        .join("\n")
      );
  }
  Ok(())
}
