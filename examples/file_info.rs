use hw::file_info::file_info_query;
#[tokio::main]
async fn main() -> e_utils::AnyResult<()> {
  #[cfg(feature = "file-info")]
  {
    // 复制文件到指定目录
    let src = "target/debug/hw.exe";
    let to = "target/debug/_libs";
    hw::p(file_info_query("copy-lib", &[src, to]).await?);
    // 打印文件信息
    hw::p(file_info_query("print", &[src]).await?);
    // 打印文件节点
    hw::p(file_info_query("nodes", &[src]).await?);
  }
  Ok(())
}
