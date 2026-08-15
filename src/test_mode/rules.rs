//! # etest 规则引擎
//!
//! 生产测试接入 etest 测试平台：hw 读取规则文件，逐个执行测试模式并输出
//! 与平台对齐的结果 JSON（外层仍由 main 包装为 R<...>R 协议）。
//!
//! 规则文件（JSON）：
//!
//!     {
//!       "name": "网卡产测",
//!       "rules": [
//!         { "id": "net-up", "mode": "net-speed", "metric": "Total_Rx", "min": 1000000, "secs": 5 },
//!         { "id": "cpu-temp", "mode": "temp", "metric": "CPU Package", "max": 85, "secs": 3 }
//!       ]
//!     }
//!
//! CLI：
//!
//!     hw --api Test --task run-rules --args etest-rules.json
//!     hw --api Test --task rules-template --args etest-rules.json   # 生成模板

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::{get as get_mode, register_all, Metric, MetricStat, ModeContext};

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
  pub pass: bool,
  pub message: Option<String>,
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

/// 生成规则模板文本
pub fn rules_template() -> e_utils::AnyResult<String> {
  Ok(serde_json::to_string_pretty(&RuleFile {
    name: Some("my-plan".into()),
    rules: vec![
      Rule {
        id: "net-up".into(),
        mode: "net-speed".into(),
        metric: "Total_Rx".into(),
        unit: Some("B/s".into()),
        min: Some(1000000.0),
        max: None,
        secs: 5,
        load: 0.0,
      },
      Rule {
        id: "ram-usage".into(),
        mode: "mem-usage".into(),
        metric: "RAM_Usage".into(),
        unit: Some("%".into()),
        min: None,
        max: Some(90.0),
        secs: 3,
        load: 0.0,
      },
    ],
  })?)
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

/// 执行规则文件，返回结构化报告（etest 判定 status 与逐项结果）
pub async fn run_rules(path: &str) -> e_utils::AnyResult<RulesReport> {
  register_all();
  let file = parse_rules(path)?;
  let plan = file
    .name
    .clone()
    .unwrap_or_else(|| path.to_string());
  let mut results: Vec<RuleResult> = Vec::new();
  for rule in &file.rules {
    results.push(run_one_rule(rule).await);
  }
  let status = results.iter().all(|r| r.pass);
  Ok(RulesReport { plan, status, results })
}

async fn run_one_rule(rule: &Rule) -> RuleResult {
  let base = RuleResult {
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
    pass: false,
    message: None,
  };

  let mode = match get_mode(&rule.mode) {
    Some(m) => m,
    None => {
      let mut r = base;
      r.message = Some(format!("未知模式 {}", rule.mode));
      return r;
    }
  };

  // 创建实例 + setup
  let mut inst = match mode.create() {
    Ok(i) => i,
    Err(e) => {
      let mut r = base;
      r.message = Some(format!("创建实例失败: {}", e));
      return r;
    }
  };
  if let Err(e) = inst.setup(&ModeContext::default()) {
    let mut r = base;
    r.message = Some(format!("初始化失败: {}", e));
    return r;
  }

  // 负载（规则级，仅 cpu-usage 等支持）
  #[cfg(feature = "system")]
  let load_handles = if rule.load > 0.0 {
    inst.spawn_load(&ModeContext::default(), rule.load).unwrap_or_default()
  } else {
    Vec::new()
  };
  #[cfg(not(feature = "system"))]
  let load_handles: Vec<std::thread::JoinHandle<()>> = Vec::new();

  // 采样
  let ctx = ModeContext {
    is_full: true,
    ..Default::default()
  };
  let mut stats: HashMap<String, MetricStat> = HashMap::new();
  let mut sums: HashMap<String, f64> = HashMap::new();
  let mut sum_sqs: HashMap<String, f64> = HashMap::new();
  let mut last_metrics: Vec<Metric> = Vec::new();
  let mut sample_err: Option<String> = None;
  for _ in 0..rule.secs.max(1) {
    tokio::time::sleep(Duration::from_secs(1)).await;
    match inst.sample(&ctx) {
      Ok(metrics) => {
        last_metrics = metrics;
        for m in &last_metrics {
          if !rule.matches(&m.name) {
            continue;
          }
          let st = stats
            .entry(m.name.clone())
            .or_insert_with(|| MetricStat::new(&m.name, m.unit.clone(), m.value));
          *sums.entry(m.name.clone()).or_insert(0.0) += m.value;
          *sum_sqs.entry(m.name.clone()).or_insert(0.0) += m.value * m.value;
          st.update(m.value);
        }
      }
      Err(e) => {
        sample_err = Some(e.to_string());
        break;
      }
    }
  }

  // 清理
  if !load_handles.is_empty() {
    crate::api_test::LOAD_CONTROLLER.stop_running();
    for h in load_handles {
      let _ = h.join();
    }
  }
  let _ = inst.teardown(&ModeContext::default());

  if let Some(err) = sample_err {
    let mut r = base;
    r.message = Some(format!("采样失败: {}", err));
    return r;
  }

  // 收尾统计
  for (name, st) in stats.iter_mut() {
    st.finish(
      sums.get(name).copied().unwrap_or(0.0),
      sum_sqs.get(name).copied().unwrap_or(0.0),
    );
  }

  // 判定：metric 匹配到的全部指标都须通过
  let matched: Vec<&MetricStat> = stats
    .values()
    .filter(|st| rule.matches(&st.name))
    .collect();
  if matched.is_empty() {
    let mut r = base;
    if let Some(first) = last_metrics.first() {
      r.metric = first.name.clone();
      r.unit = first.unit.clone();
      r.value = first.value;
    }
    r.message = Some(format!("指标未找到: {}", rule.metric));
    return r;
  }

  // 排序取第一个作为结果主体
  let mut matched: Vec<&MetricStat> = matched.into_iter().collect();
  matched.sort_by(|a, b| a.name.cmp(&b.name));
  let primary = matched[0];
  let failing: Vec<&MetricStat> = matched.iter().copied().filter(|st| !rule.passes(st)).collect();
  let pass = failing.is_empty();
  let message = if failing.is_empty() {
    None
  } else {
    Some(format!("{} 超出范围", failing.iter().map(|st| st.name.clone()).collect::<Vec<_>>().join(", ")))
  };

  RuleResult {
    item: rule.id.clone(),
    mode: rule.mode.clone(),
    metric: primary.name.clone(),
    unit: primary.unit.clone(),
    value: primary.value,
    avg: primary.avg,
    min: primary.min,
    max: primary.max,
    std_dev: primary.std_dev,
    samples: primary.samples,
    min_limit: rule.min,
    max_limit: rule.max,
    pass,
    message,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rule_matches() {
    let r = Rule {
      id: "x".into(),
      mode: "mem-usage".into(),
      metric: "RAM".into(),
      unit: None,
      min: None,
      max: Some(90.0),
      secs: 3,
      load: 0.0,
    };
    assert!(r.matches("RAM_Usage"));
    assert!(!r.matches("CPU_Usage"));
    let empty = Rule { metric: String::new(), ..r.clone() };
    assert!(empty.matches("anything"));
  }

  #[test]
  fn rule_passes_avg() {
    // avg=85 <= max 90 => 通过
    let r = Rule {
      id: "x".into(),
      mode: "m".into(),
      metric: String::new(),
      unit: None,
      min: None,
      max: Some(90.0),
      secs: 1,
      load: 0.0,
    };
    let mut st = MetricStat::new("a", "%", 80.0);
    st.update(80.0);
    st.update(90.0);
    st.finish(170.0, 80.0 * 80.0 + 90.0 * 90.0); // samples=2, avg=85
    assert!(r.passes(&st));
    let r2 = Rule { max: Some(80.0), ..r.clone() };
    assert!(!r2.passes(&st));
    let r3 = Rule { min: Some(90.0), max: None, ..r.clone() };
    assert!(!r3.passes(&st));
  }

  #[test]
  fn parse_rules_roundtrip() {
    let path = std::env::temp_dir().join("hw-etest-test.json");
    let text = rules_template().unwrap();
    std::fs::write(&path, &text).unwrap();
    let file = parse_rules(path.to_str().unwrap()).unwrap();
    assert_eq!(file.rules.len(), 2);
    assert_eq!(file.rules[0].id, "net-up");
    assert_eq!(file.rules[0].secs, 5);
    let _ = std::fs::remove_file(&path);
  }
}