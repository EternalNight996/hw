use e_utils::AnyResult;

#[tokio::main]
async fn main() -> AnyResult<()> {
  #[cfg(feature = "os-system")]
  {
    let empty: Vec<&str> = vec![];
    use hw::os_system::os_system_query;

    // 1. 检查系统激活状态（带缓存）
    hw::p("=== 检查系统激活状态（带缓存）===");
    hw::p(os_system_query("check-with-cache", &["activation_temp"]).await?);

    // 2. 检查系统激活状态
    hw::p("\n=== 检查系统激活状态 ===");
    hw::p(os_system_query("check", &empty).await?);

    // 3. 激活系统示例
    hw::p("\n=== 激活系统示例 ===");
    let activation_code = "XXXXX-XXXXX-XXXXX-XXXXX-XXXXX"; // 替换为实际的激活码
    hw::p(os_system_query("active", &[activation_code, "activation_temp"]).await?);

    // 4. 注册 KMS 服务器
    hw::p("\n=== 注册 KMS 服务器 ===");
    let kms_server = "kms.example.com"; // 替换为实际的 KMS 服务器地址
    hw::p(os_system_query("rkms", &[kms_server]).await?);

    // 5. 清除 KMS 配置
    hw::p("\n=== 清除 KMS 配置 ===");
    hw::p(os_system_query("ckms", &empty).await?);

    // 6. 查询激活缓存
    hw::p("\n=== 查询激活缓存 ===");
    hw::p(os_system_query("query-cache", &["test"]).await?);

    // 7. 清理激活缓存
    hw::p("\n=== 清理激活缓存 ===");
    hw::p(os_system_query("clean-cache", &["test"]).await?);

    // 8. 取消系统激活
    hw::p("\n=== 取消系统激活 ===");
    hw::p(os_system_query("deactive", &empty).await?);

    // 完整的激活流程示例
    hw::p("\n=== 完整激活流程示例 ===");
    let activation_process = async {
      // 1. 先检查当前激活状态
      let check_result = os_system_query("check", &empty).await?;
      hw::p(format!("当前激活状态: {}", check_result));

      // 2. 如果需要激活，先注册 KMS
      let kms_server = "kms.example.com";
      hw::p("注册 KMS 服务器...");
      os_system_query("rkms", &[kms_server]).await?;

      // 3. 执行激活
      let activation_code = "XXXXX-XXXXX-XXXXX-XXXXX-XXXXX";
      hw::p("正在激活系统...");
      os_system_query("active", &[activation_code, "activation_temp"]).await?;

      // 4. 验证激活结果
      let final_check = os_system_query("check", &empty).await?;
      hw::p(format!("激活后状态: {}", final_check));

      Ok::<_, e_utils::AnyError>(())
    };

    if let Err(e) = activation_process.await {
      hw::p(format!("激活过程出错: {}", e));
    }
  }

  Ok(())
}
