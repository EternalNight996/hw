use std::{path::Path, time::Duration};

use e_utils::{cmd::Cmd, AnyResult};
use sysinfo::{Pid, ProcessesToUpdate, System};

pub fn kill_name(name: impl AsRef<std::ffi::OsStr>) -> AnyResult<()> {
  let name = name.as_ref().to_ascii_lowercase();
  let mut sys = System::new();
  sys.refresh_processes(ProcessesToUpdate::All, true);
  for (_pid, process) in sys.processes() {
    if process.name().to_ascii_lowercase() == name {
      process.kill();
    }
  }
  Ok(())
}
pub fn query_name(name: impl AsRef<std::ffi::OsStr>) -> AnyResult<Vec<Pid>> {
  let name = name.as_ref().to_ascii_lowercase();
  let mut sys = System::new();
  sys.refresh_processes(ProcessesToUpdate::All, true);
  let mut pids: Vec<Pid> = vec![];
  for (pid, process) in sys.processes() {
    if process.name().to_ascii_lowercase() == name {
      pids.push(*pid);
    }
  }
  Ok(pids)
}
pub fn kill(pids: Vec<Pid>) -> AnyResult<()> {
  let mut sys = System::new();
  sys.refresh_processes(ProcessesToUpdate::All, true);
  for pid in pids {
    if let Some(process) = sys.process(pid) {
      process.kill();
    }
  }
  Ok(())
}

/// Run
pub fn run(name: &str, cwd: impl AsRef<Path>) -> AnyResult<Vec<Pid>> {
  let pids: Vec<Pid> = query_name(name)?;
  if !pids.is_empty() {
    crate::dp(format!("{} is already running with PIDs: {:?}", name, pids,));
    return Ok(pids);
  } else {
    let _pid = Cmd::new(name).cwd(cwd).a_spawn()?.id().map(Pid::from_u32);
    for i in 0..10 {
      let pids: Vec<Pid> = query_name(name)?;
      if !pids.is_empty() {
        return Ok(pids);
      }
      crate::wp(format!("{name} 检查进程 第 {i} 秒"));
      std::thread::sleep(Duration::from_secs(1));
    }
    Err(format!("{name} 无法运行或控制进程").into())
  }
}
