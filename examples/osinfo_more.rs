use hw::os::*;
use strum::VariantArray;
#[tokio::main]
async fn main() -> e_utils::AnyResult<()> {
  #[cfg(feature = "system")]
  {
    println!(
      "{}",
      query_os_more(Type::VARIANTS, &vec!["check-mac".to_string()], &vec!["~has_dhcp_ip".to_string()]).await
        .inspect_err(|e| eprintln!("{}", e))?
        .join("\n")
    );
  }
  Ok(())
}
