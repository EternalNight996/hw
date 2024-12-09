use e_utils::AnyResult;

#[tokio::main]
async fn main() -> AnyResult<()> {
  #[cfg(feature = "os-system")]
  {
    let empty: Vec<&str> = vec![];
    use hw::os_system::os_system_query;

    // 1. 检查系统激活状态（带缓存）
    println!("=== 检查系统激活状态（带缓存）===");
    println!("{}", os_system_query("check-with-cache", &["test"], &empty, false).await?);

    // 2. 检查系统激活状态
    println!("\n=== 检查系统激活状态 ===");
    println!("{}", os_system_query("check", &empty, &empty, false).await?);

    // 3. 激活系统示例
    println!("\n=== 激活系统示例 ===");
    let activation_code = "XXXXX-XXXXX-XXXXX-XXXXX-XXXXX"; // 替换为实际的激活码
    println!(
      "{}",
      os_system_query("active", &[activation_code, "activation_temp"], &empty, false).await?
    );

    // 4. 注册 KMS 服务器
    println!("\n=== 注册 KMS 服务器 ===");
    let kms_server = "kms.example.com"; // 替换为实际的 KMS 服务器地址
    println!("{}", os_system_query("rkms", &[kms_server], &empty, false).await?);

    // 5. 清除 KMS 配置
    println!("\n=== 清除 KMS 配置 ===");
    println!("{}", os_system_query("ckms", &empty, &empty, false).await?);

    // 6. 查询激活缓存
    println!("\n=== 查询激活缓存 ===");
    println!("{}", os_system_query("query-cache", &["test"], &empty, false).await?);

    // 7. 清理激活缓存
    println!("\n=== 清理激活缓存 ===");
    println!("{}", os_system_query("clean-cache", &["test"], &empty, false).await?);

    // 8. 取消系统激活
    println!("\n=== 取消系统激活 ===");
    println!("{}", os_system_query("deactive", &empty, &empty, false).await?);

    // 完整的激活流程示例
    println!("\n=== 完整激活流程示例 ===");
    let activation_process = async {
      // 1. 先检查当前激活状态
      let check_result = os_system_query("check", &empty, &empty, false).await?;
      println!("当前激活状态: {}", check_result);

      // 2. 如果需要激活，先注册 KMS
      let kms_server = "kms.example.com";
      println!("注册 KMS 服务器...");
      os_system_query("rkms", &[kms_server], &empty, false).await?;

      // 3. 执行激活
      let activation_code = "XXXXX-XXXXX-XXXXX-XXXXX-XXXXX";
      println!("正在激活系统...");
      os_system_query("active", &[activation_code, "activation_temp"], &empty, false).await?;

      // 4. 验证激活结果
      let final_check = os_system_query("check", &empty, &empty, false).await?;
      println!("激活后状态: {}", final_check);

      Ok::<_, e_utils::AnyError>(())
    };

    if let Err(e) = activation_process.await {
      println!("激活过程出错: {}", e);
    }
  }

  Ok(())
}
