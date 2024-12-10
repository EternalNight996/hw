use hw::os_more::{query_os_more, Type};
#[tokio::main]
async fn main() -> e_utils::AnyResult<()> {
  #[cfg(feature = "system")]
  {
    let empty: Vec<String> = vec![];
    hw::p(query_os_more(&[Type::CpuName], &empty, &[], false).await?.join("\n"));
    hw::p(query_os_more(&[Type::MemoryTotal], &empty, &[], false).await?.join("\n"));
    hw::p(query_os_more(&[Type::CpuCoreCount], &empty, &[], false).await?.join("\n"));
    hw::p(query_os_more(&[Type::OsVersion], &empty, &[], false).await?.join("\n"));
    hw::p(query_os_more(&[Type::OsFullVersion], &empty, &[], false).await?.join("\n"));
    hw::p(query_os_more(&[Type::KernelVersion], &empty, &[], false).await?.join("\n"));
    hw::p(query_os_more(&[Type::HostName], &empty, &[], false).await?.join("\n"));
    hw::p(query_os_more(&[Type::Uptime], &empty, &[], false).await?.join("\n"));
    hw::p(query_os_more(&[Type::CpuUsage], &empty, &[], false).await?.join("\n"));
    hw::p(query_os_more(&[Type::MemoryUsage], &empty, &[], false).await?.join("\n"));
    hw::p(query_os_more(&[Type::CpuArch], &empty, &[], false).await?.join("\n"));
    hw::p(query_os_more(&[Type::UserNames], &empty, &[], false).await?.join("\n"));

    // you also can take api; like memory total
    hw::p(format!("Memory API -> {}", hw::os_more::system::memory_total()?));
    hw::p("\nBase Info Done\n")
  }
  Ok(())
}
