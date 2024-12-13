use std::{path::Path, time::Duration};

use e_utils::{cmd::Cmd, AnyResult};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System, UpdateKind};

pub fn kill_name(name: impl AsRef<std::ffi::OsStr>) -> AnyResult<()> {
  let sys = System::new_with_specifics(RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing().with_cmd(UpdateKind::OnlyIfNotSet)));
  for process in sys.processes_by_name(name.as_ref()) {
    process.kill();
  }
  Ok(())
}

pub fn kill(pid: Pid) -> AnyResult<()> {
  let sys = System::new_with_specifics(RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing().with_cmd(UpdateKind::OnlyIfNotSet)));
  if let Some(process) = sys.process(pid) {
    process.kill();
  }
  Ok(())
}

/// Run
pub fn run(name: &str, cwd: impl AsRef<Path>) -> AnyResult<Vec<Pid>> {
  let mut sys = System::new();
  let _ = sys.refresh_processes(ProcessesToUpdate::All, true);
  let pids: Vec<Pid> = sys.processes_by_name(name.as_ref()).map(|v| v.pid()).collect();
  if !pids.is_empty() {
    crate::dp(format!("{} is already running with PIDs: {:?}", name, pids,));
    return Ok(pids);
  } else {
    let pid = Cmd::new(name).cwd(cwd).a_spawn()?.id().map(Pid::from_u32);
    for i in 0..10 {
      let _ = sys.refresh_processes(ProcessesToUpdate::All, true);
      match pid {
        Some(id) => match sys.process(id) {
          Some(_) => {
            crate::p(format!("Started {} with PID: {}", name, id));
            return Ok(vec![id]);
          }
          None => {}
        },
        None => {}
      }
      let pids: Vec<Pid> = sys.processes_by_name(name.as_ref()).map(|v| v.pid()).collect();
      if !pids.is_empty() {
        return Ok(pids);
      }
      crate::wp(format!("{name} 检查进程 第 {i} 秒"));
      std::thread::sleep(Duration::from_secs(1));
    }
    Err(format!("{name} 无法运行或控制进程").into())
  }
}
