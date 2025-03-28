#[tokio::main]
async fn main() -> e_utils::AnyResult<()> {
  #[cfg(all(feature = "core-temp", target_os = "windows"))]
  {
    use hw::api_test::Tester;
    use hw::{
      api_test::{HardwareType, Inner, SensorType, TestCore, TestParams, TestResults, LOAD_CONTROLLER},
      wmic::HardwareMonitor as _,
    };
    let params = TestParams {
      test_secs: 10,
      v1: 3000.0,
      v2: 5000.0,
      v3: 100.0,
    };
    let mut results = TestResults::new();
    results.hw_type = HardwareType::CPU;
    results.sensor_type = SensorType::Load;
    let mut tester = Tester {
      inner: Inner::CoreTemp(hw::core_temp::CoreTemp::new()?),
      core: TestCore {
        results,
        params,
        core_count: 0,
        is_full: true,
        is_check: false,
        is_print: false,
        is_data: true,
      },
    };
    hw::core_temp::CoreTemp::clean()?;
    let pids = hw::common::process::run("CoreTemp.exe", std::env::current_dir()?)?;
    hw::core_temp::CoreTemp::test(100)?;
    tester.core.core_count = tester.inner.get_cpu_core_count().await?;
    let load_handles = tester.spawn_load()?;
    let res = tester.run().await;
    hw::common::process::kill(pids)?;
    LOAD_CONTROLLER.stop_running();
    for handle in load_handles {
      handle.join().unwrap();
    }
    tester = res?;
    if tester.core.results.data.is_empty() && tester.core.is_check {
      tester.core.results.res = "FAIL".to_string();
      return Err(format!("{} {} 测试失败", tester.core.hw_name(), tester.core.sensor_name()).into());
    } else {
      tester.core.results.res = "PASS".to_string();
    }
    hw::p(tester.get_test_summary());
  }
  Ok(())
}
