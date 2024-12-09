#!! 在windows操作系统中, 部分功能依赖 devcon.exe

#[tokio::main]
async fn main() -> e_utils::AnyResult<()> {
  #[cfg(feature = "drive")]
  {
    use hw::drive::drive_query;

    // 1. 扫描系统驱动
    println!("=== 扫描系统驱动 ===");
    println!("{}", drive_query("scan", &[""], &[], false).await?);

    // 2. 查看特定驱动节点信息
    println!("\n=== 查看 USB 驱动节点 ===");
    println!(
      "{}",
      drive_query(
        "nodes",
        &["USB"], // 查找USB相关驱动
        &[],
        true
      )
      .await?
    );

    // 3. 打印驱动详细信息
    println!("\n=== 打印显卡驱动信息 ===");
    println!(
      "{}",
      drive_query(
        "print",
        &[], // 查找显卡驱动
        &["=net"],
        true
      )
      .await?
    );

    // 4. 添加驱动示例
    println!("\n=== 添加驱动示例 ===");
    let inf_path = "C:\\Drivers\\example.inf";
    if std::path::Path::new(inf_path).exists() {
      println!("{}", drive_query("add", &[], &[inf_path], false).await?);
    }

    // 5. 批量添加驱动文件夹
    println!("\n=== 批量添加驱动 ===");
    let driver_folder = "C:\\Drivers";
    if std::path::Path::new(driver_folder).exists() {
      println!(
        "{}",
        drive_query(
          "add-file",
          &[driver_folder],
          &[""], // 占位符
          false
        )
        .await?
      );
    }

    // 6. 启用/禁用驱动示例
    println!("\n=== 禁用/启用 USB 驱动示例 ===");
    let usb_args = ["USB"];

    // 禁用
    println!("正在禁用 USB 驱动...");
    println!("{}", drive_query("disable", &usb_args, &[], false).await?);

    // 等待几秒
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // 启用
    println!("正在启用 USB 驱动...");
    println!("{}", drive_query("enable", &usb_args, &[], false).await?);

    // 7. 导出驱动
    println!("\n=== 导出驱动示例 ===");
    println!("{}", drive_query("export", &["C:\\DriverBackup"], &[], false).await?);

    // 8. 重启特定驱动
    println!("\n=== 重启网卡驱动 ===");
    println!("{}", drive_query("restart", &["NET"], &[], false).await?);
  }
  Ok(())
}
