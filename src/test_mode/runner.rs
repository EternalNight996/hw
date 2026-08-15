//! 通用测试运行器：data / print / check 三种任务

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::{ModeContext, TestMode};

/// 测试参数（与既有 CLI 契约一致：`-- <测试秒数> <目标值> <允许误差> <负载百分比>`）
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub struct TestParams {
  /// 测试秒数（采样次数，默认 1）
  pub test_secs: usize,
  /// v1: 目标值
  pub v1: f64,
  /// v2: 允许误差 ±
  pub v2: f64,
  /// v3: 负载百分比（0 = 不加载）
  pub v3: f64,
}

impl TestParams {
  /// 从 CLI 尾随参数解析（缺省与既有 Tester 一致：1, 0.0, 0.0, 0.0）
  pub fn from_command(command: &[String]) -> Self {
    Self {
      test_secs: command.get(0).and_then(|v| v.parse().ok()).unwrap_or(1).max(1),
      v1: command.get(1).and_then(|v| v.parse().ok()).unwrap_or(0.0),
      v2: command.get(2).and_then(|v| v.parse().ok()).unwrap_or(0.0),
      v3: command.get(3).and_then(|v| v.parse().ok()).unwrap_or(0.0),
    }
  }
  /// 目标值下限（v1 - v2）
  pub fn range_min(&self) -> f64 { self.v1 - self.v2 }
  /// 目标值上限（v1 + v2）
  pub fn range_max(&self) -> f64 { self.v1 + self.v2 }
  /// 是否在允许范围内
  pub fn in_range(&self, value: f64) -> bool {
    value >= self.range_min() && value <= self.range_max()
  }
}

/// 单个指标跨采样统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricStat {
  pub name: String,
  pub unit: String,
  /// 最后一次采样值
  pub value: f64,
  pub min: f64,
  pub max: f64,
  pub avg: f64,
  pub std_dev: f64,
  pub samples: usize,
  /// check 模式下的判定结果（true=通过，false=超范围，None=未校验）
  pub pass: Option<bool>,
}

impl MetricStat {
  /// 以首个样本值创建统计
  pub fn new(name: &str, unit: impl Into<String>, value: f64) -> Self {
    Self { name: name.into(), unit: unit.into(), value, min: value, max: value, avg: value, std_dev: 0.0, samples: 0, pass: None }
  }
  /// 累加一个样本
  pub fn update(&mut self, value: f64) {
    self.min = self.min.min(value);
    self.max = self.max.max(value);
    self.value = value;
    self.samples += 1;
  }
  /// 依据样本和/平方和完成均值与标准差计算
  pub fn finish(&mut self, sum: f64, sum_sq: f64) {
    if self.samples > 0 {
      self.avg = sum / self.samples as f64;
    }
    if self.samples > 1 {
      let n = self.samples as f64;
      let var = ((sum_sq - sum * sum / n) / (n - 1.0)).max(0.0);
      self.std_dev = var.sqrt();
    }
  }
  /// 末值是否超出允许范围
  pub fn out_of_range(&self, p: TestParams) -> bool {
    !p.in_range(self.value)
  }
}

/// 负载统计（check 且 v3>0 时记录全局 CPU 使用率）
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct LoadStat {
  pub min: f64,
  pub max: f64,
  pub avg: f64,
  pub samples: usize,
  pub total: f64,
}

impl LoadStat {
  fn push(&mut self, v: f64) {
    if self.samples == 0 { self.min = v; } else { self.min = self.min.min(v); }
    self.max = self.max.max(v);
    self.total += v;
    self.samples += 1;
    self.avg = self.total / self.samples as f64;
  }
}

/// 模式运行报告（print / check 输出）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeReport {
  pub mode: String,
  pub task: String,
  /// true = 通过（check）/ 采样成功（print）
  pub status: bool,
  pub params: TestParams,
  pub metrics: Vec<MetricStat>,
  pub samples: usize,
  pub load: LoadStat,
  pub error: Option<String>,
}

/// 名称过滤：filter 为空 = 全部通过；否则名字包含任一 token 才通过
fn filter_match(name: &str, filter: &[String]) -> bool {
  filter.is_empty() || filter.iter().any(|f| name.contains(f.as_str()))
}

/// 运行一个模式；无论成败都会执行 teardown 与负载停止
pub async fn run_mode(mode: &dyn TestMode, ctx: &ModeContext<'_>, task: &str) -> e_utils::AnyResult<String> {
  let task = if task.is_empty() { "print" } else { task };
  if !matches!(task, "data" | "print" | "check") {
    return Err(format!("Task must be data/print/check, got '{}'", task).into());
  }
  let params = TestParams::from_command(ctx.command);
  let mut inst = mode.create()?;
  inst.setup(ctx)?;

  // check 且 v3>0 时启动负载
  #[cfg(feature = "system")]
  let load_handles = if task == "check" && params.v3 > 0.0 { inst.spawn_load(ctx, params.v3)? } else { vec![] };
  #[cfg(not(feature = "system"))]
  let load_handles: Vec<std::thread::JoinHandle<()>> = vec![];

  // 主流程：采样与统计（错误先收集，最后统一处理）
  let result: e_utils::AnyResult<String> = async {
    // 负载测量用 System（若启用了负载）
    #[cfg(feature = "system")]
    let mut load_sys: Option<sysinfo::System> = if load_handles.is_empty() { None } else { Some(sysinfo::System::new()) };

    let mut stats: HashMap<String, MetricStat> = HashMap::new();
    let mut sums: HashMap<String, f64> = HashMap::new();
    let mut sum_sqs: HashMap<String, f64> = HashMap::new();
    let mut consec_errors: HashMap<String, usize> = HashMap::new();
    #[allow(unused_mut)] // system 特性关闭时 load_stat 不变化
    let mut load_stat = LoadStat::default();
    let mut samples_done = 0usize;
    let mut last_metrics: Vec<super::Metric> = Vec::new();
    let mut fail_msg: Option<String> = None;

    crate::dp(format!("--- {} {} 开始: 目标 {:.1} ±{:.1}, {} 秒 ---", mode.name(), task, params.v1, params.v2, params.test_secs));

    for i in 0..params.test_secs {
      tokio::time::sleep(Duration::from_secs(1)).await;
      let metrics = inst.sample(ctx)?;
      samples_done += 1;
      last_metrics = metrics;

      #[cfg(feature = "system")]
      if let Some(sys) = load_sys.as_mut() {
        sys.refresh_cpu_usage();
        load_stat.push(sys.global_cpu_usage() as f64);
      }

      for m in &last_metrics {
        let st = stats
          .entry(m.name.clone())
          .or_insert_with(|| MetricStat::new(&m.name, m.unit.clone(), m.value));
        *sums.entry(m.name.clone()).or_insert(0.0) += m.value;
        *sum_sqs.entry(m.name.clone()).or_insert(0.0) += m.value * m.value;
        st.update(m.value);

        // check 判定
        if task == "check" && filter_match(&m.name, ctx.filter) {
          if params.in_range(m.value) {
            consec_errors.insert(m.name.clone(), 0);
          } else {
            let c = consec_errors.entry(m.name.clone()).or_insert(0);
            *c += 1;
            if *c > 2 {
              fail_msg = Some(format!("{} 连续 {} 次超出范围 (当前 {:.1}{}, 允许 {:.1}~{:.1}{})", m.name, c, m.value, m.unit, params.range_min(), params.range_max(), m.unit));
              break;
            }
          }
        }
      }
      if fail_msg.is_some() { break; }
      crate::dp(format!("--- 第 {} 秒: {}", i + 1, last_metrics.iter().map(|m| format!("{}={:.1}{}", m.name, m.value, m.unit)).collect::<Vec<_>>().join(", ")));
    }

    // 收尾统计
    for (name, st) in stats.iter_mut() {
      st.finish(sums.get(name).copied().unwrap_or(0.0), sum_sqs.get(name).copied().unwrap_or(0.0));
      if task == "check" && filter_match(name, ctx.filter) {
        st.pass = Some(!st.out_of_range(params));
      }
    }

    // data：只返回最后一次采样值（空格分隔）
    if task == "data" {
      let vals: Vec<String> = last_metrics.iter().map(|m| m.value.to_string()).collect();
      return Ok(vals.join(" "));
    }

    let mut metrics: Vec<MetricStat> = stats.into_values().collect();
    metrics.sort_by(|a, b| a.name.cmp(&b.name));

    // check：任一被校验指标末值超范围即失败
    let mut final_fail: Option<String> = fail_msg.clone();
    if final_fail.is_none() && task == "check" {
      let bad: Vec<&MetricStat> = metrics
        .iter()
        .filter(|m| filter_match(&m.name, ctx.filter))
        .filter(|m| m.pass == Some(false))
        .collect();
      if !bad.is_empty() {
        final_fail = Some(format!(
          "{} 末值超范围",
          bad.iter().map(|m| format!("{}={:.1}{}", m.name, m.value, m.unit)).collect::<Vec<_>>().join(", ")
        ));
      }
    }
    let status = final_fail.is_none();
    let report = ModeReport {
      mode: mode.name().into(),
      task: task.into(),
      status,
      params,
      metrics,
      samples: samples_done,
      load: load_stat,
      error: final_fail.clone(),
    };
    let json = serde_json::to_string(&report)?;
    crate::p(json.clone());
    crate::p(format!("--- {} 总结: {}", mode.name(), if status { "PASS" } else { "FAIL" }));
    if let Some(err) = final_fail {
      return Err(format!("{} 测试失败: {}", mode.name(), err).into());
    }
    Ok(json)
  }
  .await;

  // 无论成败：停止负载、join、teardown
  if !load_handles.is_empty() {
    crate::api_test::LOAD_CONTROLLER.stop_running();
    for h in load_handles {
      let _ = h.join();
    }
  }
  let _ = inst.teardown(ctx);
  result
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn params_parse() {
    let p = TestParams::from_command(&["5".into(), "2000".into(), "300".into(), "100".into()]);
    assert_eq!(p.test_secs, 5);
    assert_eq!(p.v1, 2000.0);
    assert_eq!(p.v2, 300.0);
    assert_eq!(p.v3, 100.0);
    assert!(p.in_range(2100.0));
    assert!(!p.in_range(2400.0));
    let d = TestParams::from_command(&[]);
    assert_eq!(d.test_secs, 1);
    assert_eq!(d.v1, 0.0);
    let z = TestParams::from_command(&["0".into()]);
    assert_eq!(z.test_secs, 1);
  }

  #[test]
  fn filter_match_works() {
    assert!(filter_match("Total_Rx", &[]));
    assert!(filter_match("Total_Rx", &["Rx".into()]));
    assert!(!filter_match("Total_Tx", &["Rx".into()]));
  }

  #[test]
  fn metric_stat_finish() {
    // new() 起始 samples=0，每次 update 计一个样本
    let mut st = MetricStat::new("a", "%", 10.0);
    st.update(20.0);
    st.update(30.0);
    // update 记录的是 20 与 30 两个样本
    st.finish(50.0, 20.0 * 20.0 + 30.0 * 30.0);
    assert_eq!(st.samples, 2); // 两次 update
    assert_eq!(st.min, 10.0);
    assert_eq!(st.max, 30.0);
    assert!((st.avg - 25.0).abs() < 1e-9); // sum/samples = 50/2
    assert!((st.std_dev - 50f64.sqrt()).abs() < 1e-9); // sqrt((1300-2500/2)/1)
  }

  #[test]
  fn metric_stat_out_of_range() {
    let p = TestParams { test_secs: 1, v1: 100.0, v2: 10.0, v3: 0.0 };
    let ok = MetricStat::new("a", "%", 105.0);
    assert!(!ok.out_of_range(p));
    let bad = MetricStat::new("a", "%", 120.0);
    assert!(bad.out_of_range(p));
  }
}