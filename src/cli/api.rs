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
    #[cfg(all(feature = "core-temp", target_os = "windows"))]
    Inner::CoreTemp(_) => {
      use crate::wmic::HardwareMonitor as _;
      if !tester.core.is_check && !tester.core.is_print && !tester.core.is_data {
        return Err("Task No check Or print Or data".into());
      }
      crate::core_temp::CoreTemp::clean()?;
      let pids = crate::common::process::run(crate::core_temp::CoreTemp::EXE, std::env::current_dir()?.join(crate::core_temp::CoreTemp::DIR))?;
      if pids.is_empty() {
        return Err(format!("Task {} is empty", crate::core_temp::CoreTemp::EXE).into());
      }
      crate::core_temp::CoreTemp::test(100)?;
      tester.core.core_count = tester.inner.get_cpu_core_count().await.unwrap_or(1);
      let load_handles = tester.spawn_load().unwrap_or_default();
      crate::dp(tester.get_test_start());
      let res = tester.run().await;
      crate::api_test::LOAD_CONTROLLER.stop_running();
      crate::common::process::kill_name(crate::core_temp::CoreTemp::EXE)?;
      crate::core_temp::CoreTemp::stop()?;
      crate::core_temp::CoreTemp::clean()?;
      for handle in load_handles {
        handle.join().map_err(|_| "CoreTemp线程错误")?;
      }
      tester = res?;
    }
    #[cfg(all(feature = "lhm", target_os = "windows"))]
    Inner::LHM(_) => {
      if !tester.core.is_check && !tester.core.is_print && !tester.core.is_data {
        return Err("Task No check Or print Or data".into());
      }
      use crate::wmic::HardwareMonitor as _;
      let pids = crate::common::process::run(crate::lhm::LHM::EXE, std::env::current_dir()?.join(crate::lhm::LHM::DIR))?;
      if pids.is_empty() {
        return Err(format!("Task {} is empty", crate::lhm::LHM::EXE).into());
      }
      crate::lhm::LHM::test(100)?;
      tester.core.core_count = tester.inner.get_cpu_core_count().await.unwrap_or(1);
      let load_handles = tester.spawn_load().unwrap_or_default();
      crate::dp(tester.get_test_start());
      let res = tester.run().await;
      crate::api_test::LOAD_CONTROLLER.stop_running();
      crate::common::process::kill_name(crate::lhm::LHM::EXE)?;
      crate::lhm::LHM::stop()?;
      crate::lhm::LHM::clean()?;
      for handle in load_handles {
        handle.join().map_err(|_| "LHM线程错误")?;
      }
      tester = res?;
    }
    #[cfg(all(feature = "ohm", target_os = "windows"))]
    Inner::OHM(_) => {
      if !tester.core.is_check && !tester.core.is_print && !tester.core.is_data {
        return Err("Task No check Or print Or data".into());
      }
      use crate::wmic::HardwareMonitor as _;
      let pids = crate::common::process::run(crate::ohm::OHM::EXE, std::env::current_dir()?.join(crate::ohm::OHM::DIR))?;
      if pids.is_empty() {
        return Err(format!("Task {} is empty", crate::ohm::OHM::EXE).into());
      }
      crate::ohm::OHM::test(100)?;
      tester.core.core_count = tester.inner.get_cpu_core_count().await.unwrap_or(1);
      let load_handles = tester.spawn_load().unwrap_or_default();
      crate::dp(tester.get_test_start());
      let res = tester.run().await;
      crate::api_test::LOAD_CONTROLLER.stop_running();
      crate::common::process::kill_name(crate::ohm::OHM::EXE)?;
      crate::ohm::OHM::stop()?;
      crate::ohm::OHM::clean()?;
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
      let pids = crate::common::process::run(crate::aida64::AIDA64::EXE, std::env::current_dir()?.join(crate::aida64::AIDA64::DIR))?;
      if pids.is_empty() {
        return Err(format!("Task {} is empty", crate::aida64::AIDA64::EXE).into());
      }
      crate::aida64::AIDA64::test(100)?;
      tester.core.core_count = tester.inner.get_cpu_core_count().await.unwrap_or(1);
      let load_handles = tester.spawn_load().unwrap_or_default();
      crate::dp(tester.get_test_start());
      let res = tester.run().await;
      crate::api_test::LOAD_CONTROLLER.stop_running();
      crate::common::process::kill_name(crate::aida64::AIDA64::EXE)?;
      crate::aida64::AIDA64::stop()?;
      crate::aida64::AIDA64::clean()?;
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
