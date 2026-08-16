//! WMI 传感器后端统一封装（OHM > LHM > AIDA64 优先级）

use crate::api_test::{HardwareType, Sensor, SensorType};
use crate::wmic::HardwareMonitor as _;

#[allow(dead_code)] // 变体是否构造取决于编译特性（默认特性下仅 OHM 生效）
pub(crate) enum Backend {
  #[cfg(feature = "ohm")]
  Ohm(crate::ohm::OHM),
  #[cfg(feature = "lhm")]
  Lhm(crate::lhm::LHM),
  #[cfg(feature = "aida64")]
  Aida64(crate::aida64::AIDA64),
}

impl Backend {
  /// 启动后端进程并建立 WMI 连接（LHM 优先——OHM 的维护版；OHM 回退；AIDA64 最后）
  pub fn start() -> e_utils::AnyResult<Backend> {
    #[cfg(feature = "lhm")]
    {
      return Self::start_one(crate::lhm::LHM::EXE, crate::lhm::LHM::DIR, || Ok(Backend::Lhm(crate::lhm::LHM::new()?)));
    }
    #[cfg(all(feature = "ohm", not(feature = "lhm")))]
    {
      return Self::start_one(crate::ohm::OHM::EXE, crate::ohm::OHM::DIR, || Ok(Backend::Ohm(crate::ohm::OHM::new()?)));
    }
    #[cfg(all(feature = "aida64", not(any(feature = "ohm", feature = "lhm"))))]
    {
      return Self::start_one(crate::aida64::AIDA64::EXE, crate::aida64::AIDA64::DIR, || Ok(Backend::Aida64(crate::aida64::AIDA64::new()?)));
    }
    #[cfg(not(any(feature = "ohm", feature = "lhm", feature = "aida64")))]
    {
      Err("no sensor backend feature enabled (lhm/ohm/aida64)".into())
    }
  }

  /// 启动后端进程、等待就绪并建立连接
  #[cfg(any(feature = "ohm", feature = "lhm", feature = "aida64"))]
  fn start_one(
    exe: &'static str,
    dir: &'static str,
    connect: impl FnOnce() -> e_utils::AnyResult<Backend>,
  ) -> e_utils::AnyResult<Backend> {
    let pids = crate::common::process::run(exe, std::env::current_dir()?.join(dir))?;
    if pids.is_empty() {
      return Err(format!("{} 进程启动失败", exe).into());
    }
    #[cfg(feature = "lhm")]
    crate::lhm::LHM::test(100)?;
    #[cfg(all(feature = "ohm", not(feature = "lhm")))]
    crate::ohm::OHM::test(100)?;
    #[cfg(all(feature = "aida64", not(any(feature = "ohm", feature = "lhm"))))]
    crate::aida64::AIDA64::test(100)?;
    connect()
  }

  /// 查询传感器
  pub fn query(&self, hw_type: HardwareType, sensor_type: SensorType) -> e_utils::AnyResult<Vec<Sensor>> {
    match self {
      #[cfg(feature = "ohm")]
      Backend::Ohm(b) => b.query(hw_type, sensor_type),
      #[cfg(feature = "lhm")]
      Backend::Lhm(b) => b.query(hw_type, sensor_type),
      #[cfg(feature = "aida64")]
      Backend::Aida64(b) => b.query(hw_type, sensor_type),
    }
  }

  /// 结束后端：杀进程、停驱动、清理驱动
  pub fn stop(&self) {
    match self {
      #[cfg(feature = "ohm")]
      Backend::Ohm(_) => {
        let _ = crate::common::process::kill_name(crate::ohm::OHM::EXE);
        let _ = crate::ohm::OHM::stop();
        let _ = crate::ohm::OHM::clean();
      }
      #[cfg(feature = "lhm")]
      Backend::Lhm(_) => {
        let _ = crate::common::process::kill_name(crate::lhm::LHM::EXE);
        let _ = crate::lhm::LHM::stop();
        let _ = crate::lhm::LHM::clean();
      }
      #[cfg(feature = "aida64")]
      Backend::Aida64(_) => {
        let _ = crate::common::process::kill_name(crate::aida64::AIDA64::EXE);
        let _ = crate::aida64::AIDA64::stop();
        let _ = crate::aida64::AIDA64::clean();
      }
    }
  }
}