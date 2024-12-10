#!! 在windows操作系统中, 部分功能依赖 devcon.exe

#[tokio::main]
async fn main() -> e_utils::AnyResult<()> {
  #[cfg(not(all(target_os = "windows", feature = "drive")))]
  {
    use hw::drive::drive_query;

    // 1. 扫描系统驱动
    hw::p("=== 扫描系统驱动 ===");
    hw::p(drive_query("scan", &[""], &[], false).await?);

    // 2. 查看特定驱动节点信息
    hw::p("\n=== 查看 USB 驱动节点 ===");
    // =net 是指devcon.exe的net类驱动  *WAN* 是匹配 id 和 name 和 driver_descript, 部分完整数据则需要true但更消耗时间
    // 可参考https://learn.microsoft.com/zh-cn/windows-hardware/drivers/devtest/devcon-findall
    hw::p(
      drive_query(
        "nodes",
        &[],
        &["=net", "*I225-V #1"],
        false, // 是否打印详细信息, 默认false：为true：数据更完整
      )
      .await?,
    );
    hw::p(
      drive_query(
        "nodes",
        &[],
        &["@pci*"], //
        true,
      )
      .await?,
    );

    // 3. 打印驱动详细信息
    hw::p("\n=== 打印显卡驱动信息 ===");
    hw::p(drive_query("print", &[], &["=net", "*WAN*"], true).await?);

    // 4. 添加驱动示例
    hw::p("\n=== 添加驱动示例 ===");
    let inf_path = "D:\\drives\\oem6.inf";
    hw::p(drive_query("add", &[inf_path, "/install"], &[], false).await?);

    // 5. 批量添加驱动文件夹
    hw::p("\n=== 批量添加驱动 ===");
    let driver_folder = "C:\\Drivers";
    hw::p(
      drive_query(
        "add-folder",
        &[driver_folder],
        &[""], // 占位符
        false,
      )
      .await?,
    );

    // 6. 启用/禁用驱动示例
    hw::p("\n=== 禁用/启用 USB 驱动示例 ===");
    let usb_args = ["USB"];

    // 禁用
    hw::p("正在禁用 USB 驱动...");
    hw::p(drive_query("disable", &usb_args, &[], false).await?);

    // 等待几秒
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // 启用
    hw::p("正在启用 USB 驱动...");
    hw::p(drive_query("enable", &usb_args, &[], false).await?);

    // 7. 导出驱动
    hw::p("\n=== 导出驱动示例 ===");
    hw::p(drive_query("export", &["oem6.inf", "C:\\DriverBackup"], &[], false).await?);
    hw::p(drive_query("export", &["oem*.inf", "C:\\DriverBackup"], &[], false).await?);

    // 8. 重启特定驱动
    hw::p("\n=== 重启网卡驱动 ===");
    hw::p(drive_query("restart", &["NET"], &[], false).await?);
  }
  Ok(())
}
