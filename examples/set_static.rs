#[tokio::main]
async fn main() -> e_utils::AnyResult<()> {
  #[cfg(feature = "network")]
  {
    use hw::os_more::{query_os_more, Type};
    hw::p(
      query_os_more(
        &[Type::NetManage],
        &["set-ip", "192.168.1.100", "255.255.255.0", "192.168.1.1"],
        &["以太网"],
        false,
      )
      .await?
      .join("\n"),
    );
    hw::p(
      query_os_more(&[Type::NetManage], &["set-dns", "223.5.5.5", "114.114.114.114"], &["以太网"], false)
        .await?
        .join("\n"),
    );
  }
  Ok(())
}
