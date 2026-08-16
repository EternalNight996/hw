//! # etest 规则引擎
//!
//! 生产测试接入 etest 测试平台：hw 读取规则文件，逐个执行测试模式并输出
//! 与平台对齐的结果 JSON（外层仍由 main 包装为 R<...>R 协议）。
//!
//! 规则文件（JSON）：
//!
//! ```text
//! {
//!   "name": "网卡产测",
//!   "rules": [
//!     { "id": "net-up", "mode": "net-speed", "metric": "Total_Rx", "min": 1000000, "secs": 5 },
//!     { "id": "cpu-temp", "mode": "temp", "metric": "CPU Package", "max": 85, "secs": 3 }
//!   ]
//! }
//! ```
//!
//! CLI：
//!
//! ```text
//! hw --api Test --task run-rules --args etest-rules.json
//! hw --api Test --task rules-template --args etest-rules.json   # 生成模板
//! ```
//!
//! GUI（hw-gui）的“规则执行”面板与 CLI 共用本引擎。

use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::{get as get_mode, register_all, Metric, MetricStat, ModeContext, ModeInstance};

/// 单条测试规则
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Rule {
  /// 测试项 ID（etest 唯一标识）
  pub id: String,
  /// 模式名（--task 同名，如 net-speed / cpu-usage / temp）
  pub mode: String,
  /// 指标名包含匹配（空 = 该模式全部指标都须通过）
  #[serde(default)]
  pub metric: String,
  /// 可选：单位校验
  #[serde(default)]
  pub unit: Option<String>,
  /// 下限（None = 不限）
  #[serde(default)]
  pub min: Option<f64>,
  /// 上限（None = 不限）
  #[serde(default)]
  pub max: Option<f64>,
  /// 稳定性：采样标准差上限（None = 不限）
  #[serde(default)]
  pub max_std: Option<f64>,
  /// 采样秒数（默认 3）
  #[serde(default = "default_secs")]
  pub secs: usize,
  /// 负载百分比（默认 0）
  #[serde(default)]
  pub load: f64,
}

fn default_secs() -> usize { 3 }

/// 规则文件
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuleFile {
  /// 可选：测试计划名
  #[serde(default)]
  pub name: Option<String>,
  pub rules: Vec<Rule>,
}

/// 规则整体失败错误：Display 输出完整报告 JSON，使 R<...>R 的 content 保持结构化
#[derive(Debug, Clone)]
pub struct RulesFail(pub String);

impl std::fmt::Display for RulesFail {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl std::error::Error for RulesFail {}

/// 单条规则执行结果（etest 对齐结构）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleResult {
  /// 测试项 ID
  pub item: String,
  pub mode: String,
  /// 参与判定的指标名（metric 匹配到多个时取第一个）
  pub metric: String,
  pub unit: String,
  /// 最后一次采样值
  pub value: f64,
  /// 采样平均值（判定用）
  pub avg: f64,
  pub min: f64,
  pub max: f64,
  pub std_dev: f64,
  pub samples: usize,
  pub min_limit: Option<f64>,
  pub max_limit: Option<f64>,
  /// 稳定性上限（标准差）
  pub max_std_limit: Option<f64>,
  pub pass: bool,
  pub message: Option<String>,
}

impl RuleResult {
  /// 构造失败结果（模式不存在 / 初始化失败 / 指标未找到等）
  pub fn failed(rule: &Rule, message: impl Into<String>) -> Self {
    RuleResult {
      item: rule.id.clone(),
      mode: rule.mode.clone(),
      metric: String::new(),
      unit: String::new(),
      value: 0.0,
      avg: 0.0,
      min: 0.0,
      max: 0.0,
      std_dev: 0.0,
      samples: 0,
      min_limit: rule.min,
      max_limit: rule.max,
      max_std_limit: rule.max_std,
      pass: false,
      message: Some(message.into()),
    }
  }
}

impl Rule {
  /// 指标是否命中本规则
  pub fn matches(&self, metric_name: &str) -> bool {
    self.metric.is_empty() || metric_name.contains(&self.metric)
  }
  /// 单个指标统计是否通过
  pub fn passes(&self, st: &MetricStat) -> bool {
    if let Some(lo) = self.min {
      if st.avg < lo {
        return false;
      }
    }
    if let Some(hi) = self.max {
      if st.avg > hi {
        return false;
      }
    }
    if let Some(ms) = self.max_std {
      if st.std_dev > ms {
        return false;
      }
    }
    true
  }
}

/// 解析规则文件
pub fn parse_rules(path: &str) -> e_utils::AnyResult<RuleFile> {
  let text = std::fs::read_to_string(path)?;
  let file: RuleFile = serde_json::from_str(&text)?;
  if file.rules.is_empty() {
    return Err("规则文件为空（rules 数组为空）".into());
  }
  Ok(file)
}

/// 全功能项规则计划（12 项：CPU 主频/温度、GPU 温度、主板温度、风扇、电压、功率、CPU/内存/磁盘/网速/GPU 利用率）
pub fn full_plan() -> RuleFile {
  RuleFile {
    name: Some("全项产测".into()),
    rules: vec![
      Rule {
        id: "cpu-clock".into(),
        mode: "cpu-clock".into(),
        metric: String::new(),
        unit: Some("MHz".into()),
        min: None,
        max: None,
        max_std: None,
        secs: 5,
        load: 0.0,
      },
      Rule {
        id: "cpu-temp".into(),
        mode: "temp".into(),
        metric: "CPU Package".into(),
        unit: Some("°C".into()),
        min: None,
        max: Some(85.0),
        max_std: None,
        secs: 3,
        load: 0.0,
      },
      Rule {
        id: "gpu-temp".into(),
        mode: "temp".into(),
        metric: "GPU".into(),
        unit: Some("°C".into()),
        min: None,
        max: Some(90.0),
        max_std: None,
        secs: 3,
        load: 0.0,
      },
      Rule {
        id: "mobo-temp".into(),
        mode: "temp".into(),
        metric: "Mainboard".into(),
        unit: Some("°C".into()),
        min: None,
        max: Some(60.0),
        max_std: None,
        secs: 3,
        load: 0.0,
      },
      Rule {
        id: "fan".into(),
        mode: "fan-speed".into(),
        metric: String::new(),
        unit: Some("RPM".into()),
        min: Some(500.0),
        max: None,
        max_std: None,
        secs: 3,
        load: 0.0,
      },
      Rule {
        id: "voltage".into(),
        mode: "voltage".into(),
        metric: String::new(),
        unit: Some("V".into()),
        min: None,
        max: None,
        max_std: None,
        secs: 3,
        load: 0.0,
      },
      Rule {
        id: "power".into(),
        mode: "power".into(),
        metric: String::new(),
        unit: Some("W".into()),
        min: None,
        max: None,
        max_std: None,
        secs: 3,
        load: 0.0,
      },
      Rule {
        id: "cpu-usage".into(),
        mode: "cpu-usage".into(),
        metric: "CPU_Usage_Global".into(),
        unit: Some("%".into()),
        min: None,
        max: Some(100.0),
        max_std: None,
        secs: 3,
        load: 0.0,
      },
      Rule {
        id: "ram-usage".into(),
        mode: "mem-usage".into(),
        metric: "RAM_Usage".into(),
        unit: Some("%".into()),
        min: None,
        max: Some(90.0),
        max_std: Some(2.0),
        secs: 3,
        load: 0.0,
      },
      Rule {
        id: "disk-c".into(),
        mode: "disk-usage".into(),
        metric: "C: Used%".into(),
        unit: Some("%".into()),
        min: None,
        max: Some(90.0),
        max_std: None,
        secs: 3,
        load: 0.0,
      },
      Rule {
        id: "net-up".into(),
        mode: "net-speed".into(),
        metric: "Total_Rx".into(),
        unit: Some("B/s".into()),
        min: Some(1000000.0),
        max: None,
        max_std: None,
        secs: 5,
        load: 0.0,
      },
      Rule {
        id: "gpu-usage".into(),
        mode: "gpu-usage".into(),
        metric: String::new(),
        unit: Some("%".into()),
        min: None,
        max: Some(100.0),
        max_std: None,
        secs: 3,
        load: 0.0,
      },
    ],
  }
}

/// 生成裸规则模板文本（兼容单规则文件）
pub fn rules_template() -> e_utils::AnyResult<String> {
  Ok(serde_json::to_string_pretty(&full_plan())?)
}

/// 从文件解析规则：优先统一配置（HwConfig.plan），兼容裸规则文件
pub fn parse_rules_file(path: &str) -> e_utils::AnyResult<RuleFile> {
  let text = std::fs::read_to_string(path)?;
  if let Ok(hc) = serde_json::from_str::<crate::gui_config::HwConfig>(&text) {
    if !hc.plan.rules.is_empty() {
      return Ok(hc.plan);
    }
  }
  let file: RuleFile = serde_json::from_str(&text)?;
  if file.rules.is_empty() {
    return Err("规则为空（rules 数组为空）".into());
  }
  Ok(file)
}

/// 规则执行报告（etest 对齐结构）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulesReport {
  /// 测试计划名（规则文件 name 或文件路径）
  pub plan: String,
  /// 整体通过（全部规则 pass）——映射到 R<...>R 的 status
  pub status: bool,
  pub results: Vec<RuleResult>,
}

impl RulesReport {
  pub fn to_json(&self) -> e_utils::AnyResult<String> {
    Ok(serde_json::to_string(self)?)
  }
}

/// 单条规则的可增量执行器（CLI 与 GUI 共用；实例不 Send，仅在同一任务内使用）
pub struct RuleRun {
  pub rule: Rule,
  /// 时间序列样本（GUI 画图用）
  pub samples: Vec<(f64, Vec<Metric>)>,
  last_metrics: Vec<Metric>,
  stats: HashMap<String, MetricStat>,
  sums: HashMap<String, f64>,
  sum_sqs: HashMap<String, f64>,
  sample_err: Option<String>,
  t0: Instant,
  inst: Option<Box<dyn ModeInstance>>,
  load_handles: Vec<std::thread::JoinHandle<()>>,
  done: bool,
}

impl RuleRun {
  /// 创建并 setup（未知模式 / 初始化失败返回 Err）
  pub fn new(rule: &Rule) -> e_utils::AnyResult<Self> {
    let mode = get_mode(&rule.mode).ok_or_else(|| format!("未知模式 {}", rule.mode))?;
    let mut inst = mode.create()?;
    inst.setup(&ModeContext::default())?;
    #[cfg(feature = "system")]
    let load_handles = if rule.load > 0.0 {
      inst.spawn_load(&ModeContext::default(), rule.load).unwrap_or_default()
    } else {
      Vec::new()
    };
    #[cfg(not(feature = "system"))]
    let load_handles: Vec<std::thread::JoinHandle<()>> = Vec::new();
    Ok(RuleRun {
      rule: rule.clone(),
      samples: Vec::new(),
      last_metrics: Vec::new(),
      stats: HashMap::new(),
      sums: HashMap::new(),
      sum_sqs: HashMap::new(),
      sample_err: None,
      t0: Instant::now(),
      inst: Some(inst),
      load_handles,
      done: false,
    })
  }

  /// 采样一步；返回 true 表示本规则已完成（达到 secs 或出错）
  pub fn step(&mut self) -> bool {
    if self.done {
      return true;
    }
    let t = self.t0.elapsed().as_secs_f64();
    let ctx = ModeContext {
      is_full: true,
      ..Default::default()
    };
    match self.inst.as_mut().expect("inst").sample(&ctx) {
      Ok(metrics) => {
        self.last_metrics = metrics.clone();
        self.samples.push((t, metrics.clone()));
        for m in &metrics {
          if !self.rule.matches(&m.name) {
            continue;
          }
          let st = self
            .stats
            .entry(m.name.clone())
            .or_insert_with(|| MetricStat::new(&m.name, m.unit.clone(), m.value));
          *self.sums.entry(m.name.clone()).or_insert(0.0) += m.value;
          *self.sum_sqs.entry(m.name.clone()).or_insert(0.0) += m.value * m.value;
          st.update(m.value);
        }
      }
      Err(e) => {
        self.sample_err = Some(e.to_string());
        self.finish();
        return true;
      }
    }
    if self.samples.len() >= self.rule.secs.max(1) {
      self.finish();
      return true;
    }
    false
  }

  /// 中止当前规则（结果标记为失败）
  pub fn cancel(&mut self) {
    if !self.done {
      self.sample_err = Some("已中止".into());
      self.finish();
    }
  }

  fn finish(&mut self) {
    if self.done {
      return;
    }
    for (name, st) in self.stats.iter_mut() {
      st.finish(
        self.sums.get(name).copied().unwrap_or(0.0),
        self.sum_sqs.get(name).copied().unwrap_or(0.0),
      );
    }
    if !self.load_handles.is_empty() {
      crate::api_test::LOAD_CONTROLLER.stop_running();
      for h in self.load_handles.drain(..) {
        let _ = h.join();
      }
    }
    if let Some(mut inst) = self.inst.take() {
      let _ = inst.teardown(&ModeContext::default());
    }
    self.done = true;
  }

  /// 汇总为规则结果
  pub fn result(&self) -> RuleResult {
    if let Some(err) = &self.sample_err {
      let mut r = RuleResult::failed(&self.rule, format!("采样失败: {}", err));
      if let Some(first) = self.last_metrics.first() {
        r.metric = first.name.clone();
        r.unit = first.unit.clone();
        r.value = first.value;
      }
      return r;
    }
    let mut matched: Vec<&MetricStat> = self
      .stats
      .values()
      .filter(|st| self.rule.matches(&st.name))
      .collect();
    if matched.is_empty() {
      let mut r = RuleResult::failed(&self.rule, format!("指标未找到: {}", self.rule.metric));
      if let Some(first) = self.last_metrics.first() {
        r.metric = first.name.clone();
        r.unit = first.unit.clone();
        r.value = first.value;
      }
      return r;
    }
    matched.sort_by(|a, b| a.name.cmp(&b.name));
    let primary = matched[0];
    let failing: Vec<&MetricStat> = matched.iter().copied().filter(|st| !self.rule.passes(st)).collect();
    let pass = failing.is_empty();
    let message = if failing.is_empty() {
      None
    } else {
      Some(format!("{} 超出范围", failing.iter().map(|st| st.name.clone()).collect::<Vec<_>>().join(", ")))
    };
    RuleResult {
      item: self.rule.id.clone(),
      mode: self.rule.mode.clone(),
      metric: primary.name.clone(),
      unit: primary.unit.clone(),
      value: primary.value,
      avg: primary.avg,
      min: primary.min,
      max: primary.max,
      std_dev: primary.std_dev,
      samples: primary.samples,
      min_limit: self.rule.min,
      max_limit: self.rule.max,
      max_std_limit: self.rule.max_std,
      pass,
      message,
    }
  }

  pub fn last_metrics(&self) -> &[Metric] {
    &self.last_metrics
  }
  pub fn is_done(&self) -> bool {
    self.done
  }
}

/// 执行单条规则（异步包装，1 秒一拍）
pub async fn run_rule(rule: &Rule) -> RuleResult {
  let mut run = match RuleRun::new(rule) {
    Ok(r) => r,
    Err(e) => return RuleResult::failed(rule, format!("初始化失败: {}", e)),
  };
  loop {
    tokio::time::sleep(Duration::from_secs(1)).await;
    if run.step() {
      break;
    }
  }
  run.result()
}

/// 执行规则文件，返回结构化报告（etest 判定 status 与逐项结果）
pub async fn run_rules(path: &str) -> e_utils::AnyResult<RulesReport> {
  register_all();
  let file = parse_rules_file(path)?;
  let plan = file.name.clone().unwrap_or_else(|| path.to_string());
  let mut results: Vec<RuleResult> = Vec::new();
  for rule in &file.rules {
    results.push(run_rule(rule).await);
  }
  let status = results.iter().all(|r| r.pass);
  Ok(RulesReport { plan, status, results })
}

#[cfg(test)]
mod tests {
  use super::*;

  /// 可控的假模式（供 RuleRun 单测）
  struct Dummy;
  struct DummyInst {
    count: usize,
    values: Vec<f64>,
  }
  impl super::super::TestMode for Dummy {
    fn name(&self) -> &'static str {
      "dummy-rule"
    }
    fn description(&self) -> &'static str {
      "dummy"
    }
    fn create(&self) -> e_utils::AnyResult<Box<dyn ModeInstance>> {
      Ok(Box::new(DummyInst { count: 0, values: vec![10.0, 20.0, 30.0] }))
    }
  }
  impl ModeInstance for DummyInst {
    fn sample(&mut self, _ctx: &ModeContext) -> e_utils::AnyResult<Vec<Metric>> {
      let v = self.values.get(self.count).copied().unwrap_or(30.0);
      self.count += 1;
      Ok(vec![Metric::new("DummyValue", v, "x")])
    }
  }

  fn dummy_rule(max: Option<f64>) -> Rule {
    Rule {
      id: "d".into(),
      mode: "dummy-rule".into(),
      metric: "DummyValue".into(),
      unit: None,
      min: None,
      max,
      max_std: None,
      secs: 2,
      load: 0.0,
    }
  }

  #[test]
  fn rule_run_incremental() {
    crate::test_mode::register(&Dummy);
    let mut run = RuleRun::new(&dummy_rule(Some(25.0))).unwrap();
    // 第一次 step：10（未完成）；第二次 step：20，samples=2 == secs => 完成
    assert!(!run.step());
    assert!(run.step());
    assert!(run.is_done());
    assert!(run.step()); // 已完成幂等
    let r = run.result();
    assert_eq!(r.samples, 2);
    assert!((r.avg - 15.0).abs() < 1e-9); // (10+20)/2
    assert!(r.pass); // 15 <= 25
  }

  #[test]
  fn rule_run_stability() {
    crate::test_mode::register(&Dummy);
    // values 10/20/30，std_dev=10；max_std=5 => 不稳定 FAIL
    let mut r = dummy_rule(None);
    r.max_std = Some(5.0);
    let mut run = RuleRun::new(&r).unwrap();
    run.step();
    run.step();
    assert!(!run.result().pass);
    // max_std=50 => PASS
    r.max_std = Some(50.0);
    let mut run = RuleRun::new(&r).unwrap();
    run.step();
    run.step();
    assert!(run.result().pass);
  }

  #[test]
  fn rule_run_fail_avg() {
    crate::test_mode::register(&Dummy);
    let mut run = RuleRun::new(&dummy_rule(Some(12.0))).unwrap();
    run.step();
    run.step();
    let r = run.result();
    assert!(!r.pass); // avg 15 > 12
    assert!(r.message.is_some());
  }

  #[test]
  fn rule_run_cancel() {
    crate::test_mode::register(&Dummy);
    let mut run = RuleRun::new(&dummy_rule(None)).unwrap();
    run.cancel();
    assert!(run.is_done());
    let r = run.result();
    assert!(!r.pass);
    assert!(r.message.is_some());
  }

  #[test]
  fn rule_matches() {
    let r = Rule {
      id: "x".into(),
      mode: "mem-usage".into(),
      metric: "RAM".into(),
      unit: None,
      min: None,
      max: Some(90.0),
      max_std: None,
      secs: 3,
      load: 0.0,
    };
    assert!(r.matches("RAM_Usage"));
    assert!(!r.matches("CPU_Usage"));
    let empty = Rule { metric: String::new(), ..r.clone() };
    assert!(empty.matches("anything"));
  }

  #[test]
  fn parse_rules_roundtrip() {
    let path = std::env::temp_dir().join("hw-etest-test.json");
    let text = rules_template().unwrap();
    std::fs::write(&path, &text).unwrap();
    let file = parse_rules(path.to_str().unwrap()).unwrap();
    assert_eq!(file.rules.len(), 12); // 全功能项模板
    assert_eq!(file.rules[0].id, "cpu-clock");
    assert_eq!(file.rules[0].secs, 5);
    // 覆盖 CPU 主频/温度/风扇等关键项
    let ids: Vec<&str> = file.rules.iter().map(|r| r.id.as_str()).collect();
    assert!(ids.contains(&"cpu-temp"));
    assert!(ids.contains(&"fan"));
    assert!(ids.contains(&"voltage"));
    assert!(ids.contains(&"power"));
    let _ = std::fs::remove_file(&path);
  }
}