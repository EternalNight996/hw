use std::str::FromStr as _;

#[allow(unused)]
use super::{Opts, OptsApi};
use crate::{
  api_test::{Inner, Tester},
  os_more,
};
use serde_json::Value;
use strum::VariantArray;

/// # Input api 统一接口
pub async fn api(op: Opts, _opts: &mut Value) -> e_utils::AnyResult<String> {
  let mut tester = Tester::from_opts(&op)?;
  match tester.inner {
    #[cfg(all(feature = "ohm", target_os = "windows"))]
    Inner::OHM(_) => {
      if !tester.core.is_check && !tester.core.is_print && !tester.core.is_data {
        return Err("Task No check Or print Or data".into());
      }
      use crate::wmic::HardwareMonitor as _;
      let pids = crate::common::process::run("OpenHardwareMonitor.exe", std::env::current_dir()?)?;
      crate::ohm::OHM::test(100)?;
      tester.core.core_count = tester.inner.get_cpu_core_count().await?;
      let load_handles = tester.spawn_load().unwrap_or_default();
      crate::dp(tester.get_test_start());
      let res = tester.run().await;
      crate::common::process::kill(pids)?;
      crate::api_test::LOAD_CONTROLLER.stop_running();
      for handle in load_handles {
        handle.join().map_err(|_| "OHM线程错误")?;
      }
      tester = res?;
    }
    #[cfg(all(feature = "aida64", target_os = "windows"))]
    Inner::AIDA64(_) => {
      if !tester.core.is_check && !tester.core.is_print && !tester.core.is_data {
        return Err("Task No check Or print Or data".into());
      }
      use crate::wmic::HardwareMonitor as _;
      let pids = crate::common::process::run("AIDA64.exe", std::env::current_dir()?)?;
      crate::aida64::AIDA64::test(100)?;
      tester.core.core_count = tester.inner.get_cpu_core_count().await?;
      let load_handles = tester.spawn_load().unwrap_or_default();
      crate::dp(tester.get_test_start());
      let res = tester.run().await;
      crate::common::process::kill(pids)?;
      crate::api_test::LOAD_CONTROLLER.stop_running();
      for handle in load_handles {
        handle.join().map_err(|_| "OHM线程错误")?;
      }
      tester = res?;
    }
    #[cfg(feature = "os")]
    Inner::OS(_) => {
      if !tester.core.is_check && !tester.core.is_print && !tester.core.is_data {
        return Err("Task No check Or print Or data".into());
      }
      tester.core.core_count = tester.inner.get_cpu_core_count().await?;
      let load_handles = tester.spawn_load().unwrap_or_default();
      crate::p(tester.get_test_start());
      let res = tester.run().await;
      crate::api_test::LOAD_CONTROLLER.stop_running();
      for handle in load_handles {
        handle.join().map_err(|_| "OHM线程错误")?;
      }
      tester = res?;
    }
    Inner::OSMore => {
      let more_type = os_more::Type::from_str(&op.task).unwrap_or_default();
      let more_types = if let os_more::Type::ALL = more_type {
        os_more::Type::VARIANTS.to_vec()
      } else {
        vec![more_type]
      };
      let res = crate::os_more::query_os_more(&more_types, &op.args, &op.command, op.full).await?.join(", ");
      return Ok(res);
    }
    Inner::Drive => return crate::drive::drive_query(&op.task, &op.args, &op.command, op.full).await,
    Inner::FileInfo => return crate::file_info::file_info_query(&op.task, &op.args).await,
    Inner::OSSystem => return crate::os_system::os_system_query(&op.task, &op.args).await,
    Inner::OSOffice => return crate::os_office::os_office_query(&op.task, &op.args).await,
    Inner::Disk => return crate::disk::disk_query(&op.task, &op.args, &op.command).await,
  };
  if tester.core.results.data.is_empty() && tester.core.is_check {
    tester.core.results.res = "FAIL".to_string();
    return Err(format!("{} {} 测试失败", tester.core.hw_name(), tester.core.sensor_name()).into());
  } else {
    tester.core.results.res = "PASS".to_string();
  }
  crate::p(tester.get_test_summary());
  if tester.core.is_data {
    Ok(tester.core.results.data)
  } else {
    Ok(serde_json::to_string(&tester.core.results)?)
  }
}
