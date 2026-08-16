//! # hw 统一配置表（hw-config.json）
//!
//! etest / 产线操作员只需编辑**一个文件** `hw-config.json` 即可调试：
//!
//! - `gui` 段：运行行为（默认视图 / 自动运行 / 总时长 / 自动关闭 / 退出码 / 负载 / 显示 / Check 参数 / 日志文件）
//! - `plan` 段：测试规则（全功能项：CPU 主频/温度、风扇、电压、功率、利用率等）
//!
//! 路径支持占位符（参考 MVCheck Conf.json）：`{origin}` = 程序目录、`{env:KEY}` = 环境变量。

use serde::{Deserialize, Serialize};

pub const CONFIG_FILE: &str = "hw-config.json";
pub const DEFAULT_LOG_FILE: &str = "hw-gui-test.log";

/// GUI 运行行为配置段
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GuiConfig {
  /// 启动视图：live | check | rules
  pub default_view: String,
  /// 启动后自动开始执行规则（plan 内嵌于统一配置）
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
  /// Check 测试参数（GUI「Check 测试」视图默认值）
  pub check_params: CheckParams,
  /// 测试结果日志文件（R<...>R 结果追加写入；空 = 不写文件）
  pub log_file: String,
}

impl Default for GuiConfig {
  fn default() -> Self {
    Self {
      default_view: "rules".into(),
      auto_run: true,
      run_seconds: 0,
      auto_close: true,
      exit_code_on_fail: true,
      raise_load_percent: 0.0,
      display_mode: "all".into(),
      display_metrics: Vec::new(),
      check_params: CheckParams::default(),
      log_file: DEFAULT_LOG_FILE.into(),
    }
  }
}

/// Check 测试参数
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

/// 配置锁：锁定后所有配置内容只读（防误改），解锁需密码
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LockConfig {
  /// 锁定开关：true = 配置只读，需解锁后才能修改
  pub enabled: bool,
  /// 解锁密码（默认 admin；明文仅为防误改，非安全措施，请勿用于高敏场景）
  pub password: String,
}

impl Default for LockConfig {
  fn default() -> Self {
    Self {
      enabled: true,
      password: "admin".into(),
    }
  }
}

/// 统一配置表：`gui` = 运行行为，`plan` = 测试规则（etest 只改这一个文件）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HwConfig {
  /// 配置说明（仅注释用途，程序忽略）
  #[serde(rename = "_说明")]
  pub note: String,
  /// 配置锁（锁定后 GUI 配置只读）
  pub lock: LockConfig,
  /// GUI 运行配置段
  pub gui: GuiConfig,
  /// 测试规则段（全功能项计划）
  pub plan: crate::test_mode::rules::RuleFile,
}

impl Default for HwConfig {
  fn default() -> Self {
    Self {
      note: "hw 统一配置表，etest 可直接修改本文件（gui=运行行为, plan=测试规则, lock=配置锁）。字段说明见 README 第 20 节。".into(),
      lock: LockConfig::default(),
      gui: GuiConfig::default(),
      plan: crate::test_mode::rules::full_plan(),
    }
  }
}

impl HwConfig {
  /// 生成统一配置模板文本
  pub fn template() -> e_utils::AnyResult<String> {
    Ok(serde_json::to_string_pretty(&HwConfig::default())?)
  }

  /// 保存统一配置到文件（UTF-8，无 BOM）
  pub fn save(&self, path: &str) -> e_utils::AnyResult<()> {
    let json = serde_json::to_string_pretty(self)?;
    std::fs::write(path, json)?;
    Ok(())
  }

  /// 加载统一配置；文件缺失/损坏时自动重建模板
  pub fn load(path: &str) -> (Self, bool) {
    match std::fs::read_to_string(path) {
      Ok(text) => match serde_json::from_str::<HwConfig>(&text) {
        Ok(mut cfg) => {
          cfg.gui.log_file = resolve_placeholders(&cfg.gui.log_file);
          (cfg, false)
        },
        Err(_) => {
          let def = HwConfig::default();
          let _ = std::fs::write(path, HwConfig::template().unwrap_or_default());
          (def, true)
        },
      },
      Err(_) => {
        let def = HwConfig::default();
        let _ = std::fs::write(path, HwConfig::template().unwrap_or_default());
        (def, true)
      },
    }
  }
}

impl GuiConfig {
  /// 显示模式是否为 single
  pub fn is_single_display(&self) -> bool {
    self.display_mode.eq_ignore_ascii_case("single") && !self.display_metrics.is_empty()
  }

  /// 指标是否可见（single 模式下按包含匹配过滤）
  pub fn metric_visible(&self, name: &str) -> bool {
    !self.is_single_display() || self.display_metrics.iter().any(|f| name.contains(f.as_str()))
  }
  
  /// 启动视图名
  pub fn view(&self) -> &str {
    self.default_view.trim()
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn default_config_template_roundtrip() {
    let t = HwConfig::template().unwrap();
    let cfg: HwConfig = serde_json::from_str(&t).unwrap();
    assert_eq!(cfg.gui.default_view, "rules");
    assert!(cfg.gui.auto_run);
    assert!(cfg.gui.auto_close);
    assert_eq!(cfg.gui.run_seconds, 0);
    assert_eq!(cfg.gui.check_params.secs, 5);
    assert_eq!(cfg.gui.log_file, DEFAULT_LOG_FILE);
    assert_eq!(cfg.plan.rules.len(), 12); // 全功能项
    assert!(cfg.lock.enabled); // 默认锁定
    assert_eq!(cfg.lock.password, "admin");
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
  fn load_creates_missing_file() {
    let path = std::env::temp_dir().join("hw-config-test.json");
    let _ = std::fs::remove_file(&path);
    let (cfg, created) = HwConfig::load(path.to_str().unwrap());
    assert!(created);
    assert!(std::path::Path::new(&path).exists());
    assert_eq!(cfg.gui.default_view, "rules");
    assert_eq!(cfg.plan.rules.len(), 12);
    let _ = std::fs::remove_file(&path);
  }

  #[test]
  fn placeholders_resolve() {
    let s = resolve_placeholders("{origin}/etest-rules.json");
    assert!(!s.starts_with("{origin}"));
    assert!(s.ends_with("/etest-rules.json"));
    let s2 = resolve_placeholders("{env:NO_SUCH_KEY_XYZ}/a");
    assert!(s2.starts_with("/a"));
    assert_eq!(resolve_placeholders("plain.json"), "plain.json");
  }
}