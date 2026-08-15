//! # hw-gui 运行配置
//!
//! etest / 产线操作员可直接编辑 `hw-gui-config.json` 控制 GUI 运行行为：
//!
//! - default_view: 启动视图（live / check / rules）
//! - rule_file: 规则文件路径（启动自动加载）
//! - auto_run: 启动后自动开始执行规则
//! - run_seconds: 测试总时长上限（秒），0 = 不限
//! - auto_close: 测试完成后自动关闭窗口
//! - exit_code_on_fail: 失败时进程退出码返回 1（供 etest 判断）
//! - raise_load_percent: 全局负载%（>0 时作为未指定负载规则的默认负载）
//! - display_mode: all（显示全部指标）/ single（只显示 display_metrics）
//! - display_metrics: display_mode=single 时的指标名/包含匹配列表

use serde::{Deserialize, Serialize};

pub const CONFIG_FILE: &str = "hw-gui-config.json";
pub const DEFAULT_LOG_FILE: &str = "hw-gui-test.log";

/// hw-gui 运行配置（JSON，etest 可直接编辑）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GuiConfig {
  /// 启动视图：live | check | rules
  pub default_view: String,
  /// 规则文件路径（启动自动加载）
  pub rule_file: String,
  /// 启动后自动开始执行规则
  pub auto_run: bool,
  /// 测试总时长上限（秒），0 = 不限（按每条规则 secs 执行）
  pub run_seconds: u64,
  /// 测试完成后自动关闭窗口
  pub auto_close: bool,
  /// 测试失败时进程退出码返回 1（仅 auto_close 时生效）
  pub exit_code_on_fail: bool,
  /// 全局负载百分比（>0 时作为未指定负载规则的默认负载）
  pub raise_load_percent: f64,
  /// 显示模式：all（全部指标）/ single（仅 display_metrics）
  pub display_mode: String,
  /// display_mode=single 时的指标名/包含匹配列表
  pub display_metrics: Vec<String>,
  /// Check 测试参数（etest 可直接修改；rules 模式以规则文件为准）
  pub check_params: CheckParams,
  /// 测试结果日志文件（R<...>R 结果追加写入；空 = 不写文件）
  pub log_file: String,
  /// 配置说明（仅注释用途，程序忽略）
  #[serde(rename = "_说明")]
  pub note: String,
}

/// Check 测试参数（GUI「Check 测试」视图的默认值）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CheckParams {
  /// 采样秒数（0 = 用内置默认 5）
  pub secs: usize,
  /// 目标值
  pub target: f64,
  /// 允许误差 ±
  pub error: f64,
  /// 负载百分比（0 = 不加载）
  pub load: f64,
}

impl Default for CheckParams {
  fn default() -> Self {
    Self {
      secs: 5,
      target: 1000.0,
      error: 500.0,
      load: 0.0,
    }
  }
}

impl Default for GuiConfig {
  fn default() -> Self {
    Self {
      default_view: "rules".into(),
      rule_file: "etest-rules.json".into(),
      auto_run: true,
      run_seconds: 0,
      auto_close: true,
      exit_code_on_fail: true,
      raise_load_percent: 0.0,
      display_mode: "all".into(),
      display_metrics: Vec::new(),
      check_params: CheckParams::default(),
      log_file: DEFAULT_LOG_FILE.into(),
      note: "hw-gui 运行配置，etest 可直接修改本文件。字段说明见 README 第 20 节。".into(),
    }
  }
}

/// 解析路径占位符（参考 MVCheck Conf.json）：{origin} = 程序目录、{env:KEY} = 环境变量
pub fn resolve_placeholders(input: &str) -> String {
  let origin = std::env::current_exe()
    .ok()
    .and_then(|p| p.parent().map(|d| d.display().to_string()))
    .unwrap_or_default();
  let mut out = input.replace("{origin}", &origin);
  while let Some(start) = out.find("{env:") {
    if let Some(rel) = out[start..].find('}') {
      let key = out[start + 5..start + rel].to_string();
      let val = std::env::var(&key).unwrap_or_default();
      out.replace_range(start..=start + rel, &val);
    } else {
      break;
    }
  }
  out
}

impl GuiConfig {
  /// 生成模板文本
  pub fn template() -> e_utils::AnyResult<String> {
    Ok(serde_json::to_string_pretty(&GuiConfig::default())?)
  }

  /// 加载配置；文件不存在时自动创建模板并返回默认值
  pub fn load(path: &str) -> (Self, bool) {
    match std::fs::read_to_string(path) {
      Ok(text) => match serde_json::from_str::<GuiConfig>(&text) {
        Ok(mut cfg) => {
          cfg.rule_file = resolve_placeholders(&cfg.rule_file);
          cfg.log_file = resolve_placeholders(&cfg.log_file);
          (cfg, false)
        },
        Err(_) => {
          let def = GuiConfig::default();
          let _ = std::fs::write(path, GuiConfig::template().unwrap_or_default());
          (def, true)
        },
      },
      Err(_) => {
        let def = GuiConfig::default();
        let _ = std::fs::write(path, GuiConfig::template().unwrap_or_default());
        (def, true)
      },
    }
  }

  /// 显示模式是否为 single
  pub fn is_single_display(&self) -> bool {
    self.display_mode.eq_ignore_ascii_case("single") && !self.display_metrics.is_empty()
  }

  /// 指标是否可见（single 模式下按包含匹配过滤）
  pub fn metric_visible(&self, name: &str) -> bool {
    !self.is_single_display() || self.display_metrics.iter().any(|f| name.contains(f.as_str()))
  }

  /// 启动视图
  pub fn view(&self) -> &str {
    self.default_view.trim()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn default_config_template_roundtrip() {
    let t = GuiConfig::template().unwrap();
    let cfg: GuiConfig = serde_json::from_str(&t).unwrap();
    assert_eq!(cfg.default_view, "rules");
    assert!(cfg.auto_run);
    assert!(cfg.auto_close);
    assert_eq!(cfg.run_seconds, 0);
    assert_eq!(cfg.check_params.secs, 5);
    assert_eq!(cfg.check_params.target, 1000.0);
    assert_eq!(cfg.log_file, DEFAULT_LOG_FILE);
  }

  #[test]
  fn single_display_filter() {
    let mut cfg = GuiConfig::default();
    cfg.display_mode = "single".into();
    cfg.display_metrics = vec!["CPU_0_Clock".into(), "Total_Rx".into()];
    assert!(cfg.is_single_display());
    assert!(cfg.metric_visible("CPU_0_Clock"));
    assert!(cfg.metric_visible("Total_Rx"));
    assert!(!cfg.metric_visible("RAM_Usage"));
    cfg.display_mode = "all".into();
    assert!(cfg.metric_visible("RAM_Usage"));
  }

  #[test]
  fn placeholders_resolve() {
    // {origin} 解析为程序目录（非空）
    let s = resolve_placeholders("{origin}/etest-rules.json");
    assert!(!s.starts_with("{origin}"));
    assert!(s.ends_with("/etest-rules.json"));
    // 未知 {env:KEY} 解析为空
    let s2 = resolve_placeholders("{env:NO_SUCH_KEY_XYZ}/a");
    assert!(s2.starts_with("/a"));
    // 无占位符原样
    assert_eq!(resolve_placeholders("plain.json"), "plain.json");
  }

  #[test]
  fn load_creates_missing_file() {
    let path = std::env::temp_dir().join("hw-gui-config-test.json");
    let _ = std::fs::remove_file(&path);
    let (cfg, created) = GuiConfig::load(path.to_str().unwrap());
    assert!(created);
    assert!(std::path::Path::new(&path).exists());
    assert_eq!(cfg.default_view, "rules");
    let _ = std::fs::remove_file(&path);
  }
}