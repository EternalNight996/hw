use crate::api_test::Tester;
#[allow(unused)]
use super::{Opts, OptsApi};
use serde_json::Value;

/// # Input api 统一接口
pub async fn api(op: Opts, _opts: &mut Value) -> e_utils::AnyResult<String> {
  let mut tester = Tester::from_opts(&op)?;
  if !tester.core.is_check && !tester.core.is_print && !tester.core.is_data {
    return Err("Task No check Or print Or data".into());
  }
  println!("{}", tester.get_test_start());
  #[cfg(any(feature = "ohm", feature = "aida64", feature = "os"))]
  match tester.inner {
    #[cfg(all(feature = "ohm", target_os = "windows"))]
    crate::api_test::Inner::OHM(_) => {
      use crate::wmic::HardwareMonitor as _;
      let pid = crate::common::process::run("OpenHardwareMonitor.exe", std::env::current_dir()?)?;
      crate::ohm::OHM::test(100)?;
      tester.core.core_count = tester.inner.get_cpu_core_count().await?;
      let load_handles = tester.spawn_load()?;
      let res = tester.run().await;
      crate::common::process::kill(pid)?;
      crate::api_test::LOAD_CONTROLLER.stop_running();
      for handle in load_handles {
        handle.join().unwrap();
      }
      tester = res?;
    }
    #[cfg(all(feature = "aida64", target_os = "windows"))]
    crate::api_test::Inner::AIDA64(_) => {
      use crate::wmic::HardwareMonitor as _;
      let pid = crate::common::process::run("AIDA64.exe", std::env::current_dir()?)?;
      crate::aida64::AIDA64::test(100)?;
      tester.core.core_count = tester.inner.get_cpu_core_count().await?;
      let load_handles = tester.spawn_load()?;
      let res = tester.run().await;
      crate::common::process::kill(pid)?;
      crate::api_test::LOAD_CONTROLLER.stop_running();
      for handle in load_handles {
        handle.join().unwrap();
      }
      tester = res?;
    }
    #[cfg(feature = "os")]
    crate::api_test::Inner::OS(_) => {
      tester.core.core_count = tester.inner.get_cpu_core_count().await?;
      let load_handles = tester.spawn_load()?;
      let res = tester.run().await;
      crate::api_test::LOAD_CONTROLLER.stop_running();
      for handle in load_handles {
        handle.join().unwrap();
      }
      tester = res?;
    }
  };
  if tester.core.results.data.is_empty() && tester.core.is_check {
    tester.core.results.res = "FAIL".to_string();
    return Err(format!("{} {} 测试失败", tester.core.hw_name(), tester.core.sensor_name()).into());
  } else {
    tester.core.results.res = "PASS".to_string();
  }
  println!("{}", tester.get_test_summary());
  if tester.core.is_data {
    Ok(tester.core.results.data)
  } else {
    Ok(serde_json::to_string(&tester.core.results)?)
  }
}
