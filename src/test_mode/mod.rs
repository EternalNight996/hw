//! # Test Mode 框架
//!
//! 受 TrafficMonitor 插件接口（IPluginItem / ITMPlugin）启发，hw 提供一套
//! 可注册的测试模式框架：新增一个测试模式 = 实现 [`TestMode`] + 注册一行。
//!
//! CLI 用法（需 cli 特性）：
//!
//! ```text
//! # 列出所有已注册模式
//! hw --api Test --task list
//! # 运行某个模式：data（当前值）/ print（统计）/ check（校验+负载）
//! hw --api Test --task net-speed --args print -- 5
//! hw --api Test --task cpu-usage --args check --filter CPU_Usage_Global -- 5 80 10 100
//! ```

mod registry;
mod runner;
pub mod builtin;
pub mod rules;

pub use registry::{get, list, register, registered_modes, Registry};
pub use runner::{run_mode, LoadStat, MetricStat, ModeReport, TestParams};
pub use rules::{Rule, RuleFile, RuleResult};

use serde::{Deserialize, Serialize};

/// 一次采样得到的指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
  /// 指标名（如 "Total_Rx"、"CPU_Usage_Global"、"C: Used%"）
  pub name: String,
  /// 数值
  pub value: f64,
  /// 单位（如 "B/s"、"%"、"MHz"、"GiB"、"°C"）
  pub unit: String,
  /// 可选的单次采样最小值（如传感器 Min）
  pub min: Option<f64>,
  /// 可选的单次采样最大值（如传感器 Max）
  pub max: Option<f64>,
}

impl Metric {
  pub fn new(name: impl Into<String>, value: f64, unit: impl Into<String>) -> Self {
    Self { name: name.into(), value, unit: unit.into(), min: None, max: None }
  }
  pub fn with_range(mut self, min: Option<f64>, max: Option<f64>) -> Self {
    self.min = min;
    self.max = max;
    self
  }
}

/// 模式运行上下文（由 CLI 参数构造，库用户可自行构造）
#[derive(Debug, Clone, Default)]
pub struct ModeContext<'a> {
  /// --args 扩展参数（模式自定义，内置模式暂未使用）
  pub args: &'a [String],
  /// --command（-- 之后的尾随参数）
  pub command: &'a [String],
  /// --filter 名称过滤（指标名 / 接口名 / 挂载点包含匹配）
  pub filter: &'a [String],
  /// --full 完整信息
  pub is_full: bool,
}

/// 测试模式（静态注册的工厂；实例本身不要求 Send，见 [`ModeInstance`]）
pub trait TestMode: Send + Sync {
  /// 模式名（CLI --task 的值）
  fn name(&self) -> &'static str;
  /// 一行描述
  fn description(&self) -> &'static str;
  /// 创建本次运行的状态实例（可持有采样状态）
  fn create(&self) -> e_utils::AnyResult<Box<dyn ModeInstance>>;
}

/// 单次运行实例（注意：WMI 后端对象非 Send，实例仅在同一任务内使用）
pub trait ModeInstance {
  /// 可选的一次性初始化（如启动 OHM/LHM 进程）
  fn setup(&mut self, _ctx: &ModeContext) -> e_utils::AnyResult<()> {
    Ok(())
  }
  /// 采样当前指标
  fn sample(&mut self, ctx: &ModeContext) -> e_utils::AnyResult<Vec<Metric>>;
  /// 可选的负载生成器（仅 check 模式且 v3>0 时调用）
  fn spawn_load(&self, _ctx: &ModeContext, _target: f64) -> e_utils::AnyResult<Vec<std::thread::JoinHandle<()>>> {
    Ok(vec![])
  }
  /// 可选的一次性清理
  fn teardown(&mut self, _ctx: &ModeContext) -> e_utils::AnyResult<()> {
    Ok(())
  }
}

/// 注册所有内置模式（在 lib 初始化时调用一次）
pub fn register_all() {
  builtin::register_all();
}

/// CLI 入口：--api Test --task <模式名|list>
///
/// 动词（data/print/check）由 --args 首参数指定，缺省为 print：
///
/// ```text
/// hw --api Test --task list
/// hw --api Test --task net-speed --args print -- 5
/// hw --api Test --task cpu-usage --args check --filter CPU_Usage_Global -- 5 80 10 100
/// ```
///
/// 首次调用时自动注册内置模式（重复调用幂等）。
#[cfg(feature = "cli")]
pub async fn run(op: &crate::cli::Opts) -> e_utils::AnyResult<String> {
  register_all();
  match op.task.as_str() {
    "list" | "" => {
      let modes = list();
      if modes.is_empty() {
        return Err("No test modes registered".into());
      }
      let mut out = String::from("Registered test modes:\n");
      for (name, desc) in modes {
        out.push_str(&format!("  {:<14} {}\n", name, desc));
      }
      crate::p(out.trim_end());
      Ok(out)
    }
    // etest 规则任务
    "run-rules" => {
      let path = op.args.first().map(|s| s.as_str()).unwrap_or(crate::gui_config::CONFIG_FILE);
      let report = rules::run_rules(path).await?;
      let json = report.to_json()?;
      if report.status {
        return Ok(json);
      }
      // 整体失败：返回内容为报告 JSON 的错误，main 会置 status=false（etest 兼容）
      return Err(rules::RulesFail(json).into());
    }
    "rules-template" => {
      let template = rules::rules_template()?;
      if let Some(path) = op.args.first() {
        std::fs::write(path, &template)?;
        crate::p(format!("已生成规则模板: {}", path));
      }
      return Ok(template);
    }
    // 统一配置表模板（gui + plan 一个文件，etest 直接改）
    "config-template" => {
      let path = op.args.first().map(|s| s.as_str()).unwrap_or(crate::gui_config::CONFIG_FILE);
      let template = crate::gui_config::HwConfig::template()?;
      std::fs::write(path, &template)?;
      crate::p(format!("已生成统一配置表: {}", path));
      return Ok(template);
    }
    task => {
      let mode = get(task).ok_or_else(|| {
        let names: Vec<&str> = list().iter().map(|(n, _)| *n).collect();
        format!("Unknown test mode '{}'. Available: {}", task, names.join(", "))
      })?;
      // 动词：--args 首参数为 data/print/check 时消费之，否则默认 print
      let (verb, mode_args) = match op.args.first().map(|s| s.as_str()) {
        Some(v) if matches!(v, "data" | "print" | "check") => (v, &op.args[1..]),
        _ => ("print", &op.args[..]),
      };
      let ctx = ModeContext {
        args: mode_args,
        command: &op.command,
        filter: &op.filter,
        is_full: op.full,
      };
      runner::run_mode(mode, &ctx, verb).await
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  struct Dummy;
  impl TestMode for Dummy {
    fn name(&self) -> &'static str { "dummy" }
    fn description(&self) -> &'static str { "dummy mode" }
    fn create(&self) -> e_utils::AnyResult<Box<dyn ModeInstance>> {
      Ok(Box::new(DummyInst))
    }
  }
  struct DummyInst;
  impl ModeInstance for DummyInst {
    fn sample(&mut self, _ctx: &ModeContext) -> e_utils::AnyResult<Vec<Metric>> {
      Ok(vec![Metric::new("v", 42.0, "x")])
    }
  }

  #[test]
  fn registry_roundtrip() {
    registry::clear_for_test();
    register(&Dummy);
    assert!(get("dummy").is_some());
    assert!(get("nope").is_none());
    assert_eq!(list().len(), 1);
    assert_eq!(list()[0].0, "dummy");
  }
}