use std::path::Path;

use e_utils::{
  cmd::{Cmd, *},
  AnyResult,
};

pub fn kill_name(name: impl AsRef<std::ffi::OsStr>) -> AnyResult<()> {
  let sys = sysinfo::System::new_with_specifics(sysinfo::RefreshKind::nothing().with_processes(
    sysinfo::ProcessRefreshKind::nothing().with_cmd(sysinfo::UpdateKind::OnlyIfNotSet),
  ));
  for process in sys.processes_by_name(name.as_ref()) {
    process.kill();
  }
  Ok(())
}

pub fn kill(pid: sysinfo::Pid) -> AnyResult<()> {
  let sys = sysinfo::System::new_with_specifics(sysinfo::RefreshKind::nothing().with_processes(
    sysinfo::ProcessRefreshKind::nothing().with_cmd(sysinfo::UpdateKind::OnlyIfNotSet),
  ));
  if let Some(process) = sys.process(pid) {
    process.kill();
  }
  Ok(())
}
pub fn run(name: &str, cwd: impl AsRef<Path>) -> AnyResult<sysinfo::Pid> {
  let mut sys = sysinfo::System::new_with_specifics(sysinfo::RefreshKind::nothing().with_processes(
    sysinfo::ProcessRefreshKind::nothing().with_cmd(sysinfo::UpdateKind::OnlyIfNotSet),
  ));
  let pid = if let Some((pid, process)) = sys
    .processes()
    .iter()
    .find(|(_, process)| process.name().to_ascii_lowercase() == *name.to_ascii_lowercase())
  {
    println!(
      "{} is already running with Name: {}, PID: {}",
      name,
      process.name().to_string_lossy(),
      pid
    );
    pid.clone()
  } else {
    let pid = Cmd::new(name)
      .cwd(cwd)
      .set_type(ExeType::WindowsExe)
      .a_spawn()?
      .id()
      .ok_or("无法解析Proccess ID")?;
    // 启动新进程
    let _ = sys.refresh_processes(sysinfo::ProcessesToUpdate::All, false);
    if let Some(process) = sys.process(sysinfo::Pid::from_u32(pid)) {
      println!("Started {} with PID: {}", name, pid);
      process.pid()
    } else {
      return Err(format!("无法找到进程 {}", pid).into());
    }
  };
  Ok(pid)
}
