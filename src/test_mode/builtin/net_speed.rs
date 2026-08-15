//! 网速上/下行测试模式（对应 TrafficMonitor 核心监控项）
//!
//! 指标：Total_Rx / Total_Tx（B/s，全接口汇总）；--full 或 --filter 时输出各接口速率。
//!
//!     hw --api Test --task net-speed --task print -- 5
//!     hw --api Test --task net-speed --task check --filter Total_Rx -- 5 1000000 200000

use std::collections::HashMap;

use crate::test_mode::{Metric, ModeContext, ModeInstance, TestMode};

pub static NET_SPEED: NetSpeedMode = NetSpeedMode;

pub struct NetSpeedMode;

impl TestMode for NetSpeedMode {
  fn name(&self) -> &'static str {
    "net-speed"
  }
  fn description(&self) -> &'static str {
    "Network upload/download rate (B/s), total and per-interface"
  }
  fn create(&self) -> e_utils::AnyResult<Box<dyn ModeInstance>> {
    Ok(Box::new(NetSpeed::new()))
  }
}

struct NetSpeed {
  nets: sysinfo::Networks,
  /// 上次采样时各接口累计字节 (rx, tx)
  prev: HashMap<String, (u64, u64)>,
}

impl NetSpeed {
  fn new() -> Self {
    Self {
      nets: sysinfo::Networks::new_with_refreshed_list(),
      prev: HashMap::new(),
    }
  }
}

impl ModeInstance for NetSpeed {
  fn sample(&mut self, ctx: &ModeContext) -> e_utils::AnyResult<Vec<Metric>> {
    self.nets.refresh(true);
    let mut total_rx = 0u64;
    let mut total_tx = 0u64;
    let mut metrics: Vec<Metric> = Vec::new();

    let mut ifaces: Vec<(String, f64, f64)> = Vec::new();
    for (name, data) in self.nets.list() {
      let rx = data.total_received();
      let tx = data.total_transmitted();
      let (prx, ptx) = self.prev.get(name).copied().unwrap_or((rx, tx));
      let rx_rate = rx.saturating_sub(prx) as f64;
      let tx_rate = tx.saturating_sub(ptx) as f64;
      self.prev.insert(name.clone(), (rx, tx));
      total_rx += rx_rate as u64;
      total_tx += tx_rate as u64;
      ifaces.push((name.clone(), rx_rate, tx_rate));
    }

    metrics.push(Metric::new("Total_Rx", total_rx as f64, "B/s"));
    metrics.push(Metric::new("Total_Tx", total_tx as f64, "B/s"));

    if ctx.is_full || !ctx.filter.is_empty() {
      for (name, rx, tx) in ifaces {
        let pass = ctx.filter.is_empty() || ctx.filter.iter().any(|f| name.contains(f.as_str()));
        if pass {
          metrics.push(Metric::new(format!("{}_Rx", name), rx, "B/s"));
          metrics.push(Metric::new(format!("{}_Tx", name), tx, "B/s"));
        }
      }
    }
    Ok(metrics)
  }
}