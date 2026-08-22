#![cfg(feature = "gui")]
//! hw 图形化界面（hw-gui）：实时监控 + check 测试可视化 + 历史/报告导出
//!
//! 构建：cargo build --features gui --bin hw-gui
//! 运行：target\debug\hw-gui.exe
//! 自动化冒烟测试：设置环境变量 HW_GUI_SMOKE=1 时 3 秒后自动关闭窗口

use std::collections::HashMap;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use eframe::egui;
use egui_plot::{HLine, Legend, Line, Plot, PlotPoints};

use hw::gui_config::{HwConfig, CONFIG_FILE};
use hw::test_mode::{get as get_mode, list as list_modes, register_all, rules, Metric, MetricStat, ModeContext, ModeInstance, TestParams};

const SAMPLE_INTERVAL: Duration = Duration::from_secs(1);
const MAX_POINTS: usize = 900;

/// 进程退出码（auto_close 时由测试结果决定，供 etest 判断）
static EXIT_CODE: AtomicI32 = AtomicI32::new(0);

fn main() -> eframe::Result {
  hw::log::init_logging();
  register_all();
  let smoke = std::env::var("HW_GUI_SMOKE").is_ok();
  let opts = eframe::NativeOptions {
    viewport: egui::ViewportBuilder::default()
      .with_inner_size([1280.0, 840.0])
      .with_min_inner_size([960.0, 600.0])
      .with_title("HW Monitor GUI"),
    ..Default::default()
  };
  let result = eframe::run_native("HW Monitor GUI", opts, Box::new(|cc| Ok(Box::new(GuiApp::new(cc, smoke)))));
  // auto_close 场景：测试失败时以退出码 1 结束，供 etest 判断
  if result.is_ok() {
    let code = EXIT_CODE.load(Ordering::SeqCst);
    if code != 0 {
      std::process::exit(code);
    }
  }
  result
}

fn matches_filter(filter: &[String], name: &str) -> bool {
  filter.is_empty() || filter.iter().any(|f| name.contains(f.as_str()))
}

/// etest 规则执行面板状态（与 CLI run-rules 共用 rules::RuleRun）
struct RulesGui {
  file: Option<rules::RuleFile>,
  /// 测试模式总开关（来自 config.test_mode）
  global_test_mode: bool,
  run: Option<rules::RuleRun>,
  results: Vec<rules::RuleResult>,
  plan: String,
  done: bool,
  msg: String,
  global_load: f64,
  run_started: Option<Instant>,
}

impl RulesGui {
  fn new() -> Self {
    Self {
      file: None,
      global_test_mode: true,
      run: None,
      results: Vec::new(),
      plan: String::new(),
      done: false,
      msg: String::new(),
      global_load: 0.0,
      run_started: None,
    }
  }

  fn total(&self) -> usize {
    self.file.as_ref().map(|f| f.rules.len()).unwrap_or(0)
  }

  fn start(&mut self) {
    if self.file.is_none() {
      self.msg = "请先加载规则文件".into();
      return;
    }
    self.results.clear();
    self.run = None;
    self.done = false;
    self.run_started = Some(Instant::now());
    self.advance();
  }

  /// 整体通过：跳过项（enabled=false）不参与判定
  fn overall_ok(&self) -> bool {
    self.results.iter().filter(|r| !r.skipped).all(|r| r.pass)
  }

  /// 推进到下一条规则；全部完成时置 done
  fn advance(&mut self) {
    let total = self.total();
    if self.results.len() >= total {
      let ok = self.overall_ok();
      let plan = self.plan.clone();
      self.msg = format!("{} 完成：{} / {}", plan, if ok { "PASS" } else { "FAIL" }, self.results.len());
      self.done = true;
      return;
    }
    let idx = self.results.len();
    let mut rule = self.file.as_ref().unwrap().rules[idx].clone();
    // enabled=false 或 总开关关闭：不执行，标记跳过并继续
    if !rule.enabled || !self.global_test_mode {
      self.results.push(rules::RuleResult::skipped(&rule));
      self.advance();
      return;
    }
    // 全局负载覆盖：配置 >0 且规则未指定负载时生效
    if self.global_load > 0.0 && rule.load == 0.0 {
      rule.load = self.global_load;
    }
    match rules::RuleRun::new(&rule) {
      Ok(run) => {
        self.msg = format!("运行规则 {}/{}：{}（{}）", idx + 1, total, rule.id, rule.mode);
        self.run = Some(run);
      }
      Err(e) => {
        self.results.push(rules::RuleResult::failed(&rule, format!("初始化失败: {}", e)));
        self.advance();
      }
    }
  }

  /// 中止当前规则，剩余规则标记为未执行
  fn stop(&mut self) {
    if let Some(mut run) = self.run.take() {
      run.cancel();
      self.results.push(run.result());
    }
    while self.results.len() < self.total() {
      let rule = self.file.as_ref().unwrap().rules[self.results.len()].clone();
      if rule.enabled {
        self.results.push(rules::RuleResult::failed(&rule, "未执行（已停止）"));
      } else {
        self.results.push(rules::RuleResult::skipped(&rule));
      }
    }
    self.done = true;
    self.run_started = None;
    self.msg = "已停止".into();
  }

  /// 超时/中止：当前规则标记失败，剩余规则标记 reason
  fn timeout(&mut self, reason: &str) {
    if let Some(mut run) = self.run.take() {
      run.cancel();
      self.results.push(run.result());
    }
    while self.results.len() < self.total() {
      let rule = self.file.as_ref().unwrap().rules[self.results.len()].clone();
      if rule.enabled {
        self.results.push(rules::RuleResult::failed(&rule, reason));
      } else {
        self.results.push(rules::RuleResult::skipped(&rule));
      }
    }
    self.done = true;
    self.run_started = None;
    self.msg = reason.into();
  }

  fn export_report(&mut self) {
    let path = format!("etest-report-{}.json", now_str());
    let report = rules::RulesReport {
      plan: self.plan.clone(),
      status: self.overall_ok(),
      results: self.results.clone(),
    };
    match serde_json::to_string_pretty(&report).map(|j| std::fs::write(&path, j)) {
      Ok(Ok(())) => self.msg = format!("已导出报告: {}", path),
      _ => self.msg = "导出失败".into(),
    }
  }
}

// ---------------------------------------------------------------------------
// 实时采样器
// ---------------------------------------------------------------------------
struct LiveSampler {
  mode_name: String,
  inst: Box<dyn ModeInstance>,
  t0: Instant,
  series: HashMap<String, Vec<(f64, f64)>>,
  last_metrics: Vec<Metric>,
  error: Option<String>,
}

impl LiveSampler {
  fn step(&mut self) {
    let ctx = ModeContext {
      is_full: true,
      ..Default::default()
    };
    match self.inst.sample(&ctx) {
      Ok(metrics) => {
        let t = self.t0.elapsed().as_secs_f64();
        for m in &metrics {
          let v = self.series.entry(m.name.clone()).or_default();
          v.push((t, m.value));
          if v.len() > MAX_POINTS {
            v.remove(0);
          }
        }
        self.last_metrics = metrics;
        self.error = None;
      }
      Err(e) => self.error = Some(e.to_string()),
    }
  }
  fn stop(&mut self) {
    let _ = self.inst.teardown(&ModeContext::default());
  }
}

// ---------------------------------------------------------------------------
// check 测试运行
// ---------------------------------------------------------------------------
#[derive(PartialEq)]
enum CheckPhase {
  Running,
  Done,
}

struct CheckRun {
  mode_name: String,
  params: TestParams,
  filter: Vec<String>,
  inst: Box<dyn ModeInstance>,
  t0: Instant,
  samples: Vec<(f64, Vec<Metric>)>,
  sums: HashMap<String, f64>,
  sum_sqs: HashMap<String, f64>,
  stats: HashMap<String, MetricStat>,
  consec: HashMap<String, usize>,
  phase: CheckPhase,
  load_handles: Vec<std::thread::JoinHandle<()>>,
  error: Option<String>,
}

impl CheckRun {
  fn step(&mut self) {
    if self.phase != CheckPhase::Running {
      return;
    }
    let ctx = ModeContext {
      is_full: true,
      ..Default::default()
    };
    let t = self.t0.elapsed().as_secs_f64();
    let metrics = match self.inst.sample(&ctx) {
      Ok(m) => m,
      Err(e) => {
        self.error = Some(e.to_string());
        self.finish();
        return;
      }
    };
    self.samples.push((t, metrics.clone()));
    for m in &metrics {
      let st = self
        .stats
        .entry(m.name.clone())
        .or_insert_with(|| MetricStat::new(&m.name, m.unit.clone(), m.value));
      *self.sums.entry(m.name.clone()).or_insert(0.0) += m.value;
      *self.sum_sqs.entry(m.name.clone()).or_insert(0.0) += m.value * m.value;
      st.update(m.value);

      if matches_filter(&self.filter, &m.name) {
        if self.params.in_range(m.value) {
          self.consec.insert(m.name.clone(), 0);
        } else {
          let c = self.consec.entry(m.name.clone()).or_insert(0);
          *c += 1;
          if *c > 2 {
            self.error = Some(format!(
              "{} 连续 {} 次超出范围 (当前 {:.1}{}, 允许 {:.1}~{:.1}{})",
              m.name,
              c,
              m.value,
              m.unit,
              self.params.range_min(),
              self.params.range_max(),
              m.unit
            ));
            self.finish();
            return;
          }
        }
      }
    }
    if self.samples.len() >= self.params.test_secs.max(1) {
      self.finish();
    }
  }

  fn finish(&mut self) {
    for (name, st) in self.stats.iter_mut() {
      st.finish(self.sums.get(name).copied().unwrap_or(0.0), self.sum_sqs.get(name).copied().unwrap_or(0.0));
      if matches_filter(&self.filter, name) {
        st.pass = Some(!st.out_of_range(self.params));
      }
    }
    if !self.load_handles.is_empty() {
      hw::api_test::LOAD_CONTROLLER.stop_running();
      for h in self.load_handles.drain(..) {
        let _ = h.join();
      }
    }
    let _ = self.inst.teardown(&ModeContext::default());
    self.phase = CheckPhase::Done;
  }

  fn status_ok(&self) -> bool {
    self.error.is_none()
      && self
        .stats
        .values()
        .filter(|m| matches_filter(&self.filter, &m.name))
        .all(|m| m.pass == Some(true))
  }
}

fn now_str() -> String {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map(|d| d.as_secs().to_string())
    .unwrap_or_else(|_| "0".into())
}

// ---------------------------------------------------------------------------
// 主应用
// ---------------------------------------------------------------------------
struct GuiApp {
  modes: Vec<(String, String)>,
  selected: String,
  live: Option<LiveSampler>,
  check: Option<CheckRun>,
  // check 参数编辑
  c_secs: usize,
  c_target: f64,
  c_err: f64,
  c_load: f64,
  c_filter: String,
  last_sample: Option<Instant>,
  started_at: Instant,
  msg: String,
  smoke: bool,
  unlock_pwd: String,
  unlocked: bool,
  rules_emitted: bool,
  auto_closed: bool,
  rules: RulesGui,
  config: HwConfig,
}

impl GuiApp {
  fn new(cc: &eframe::CreationContext<'_>, smoke: bool) -> Self {
    install_cjk_fonts(&cc.egui_ctx);
    let modes: Vec<(String, String)> = list_modes().into_iter().map(|(n, d)| (n.to_string(), d.to_string())).collect();
    let selected = modes.first().map(|(n, _)| n.clone()).unwrap_or_else(|| "net-speed".into());
    let count = modes.len();
    // 加载运行配置（缺失自动创建模板）
    let (config, created) = HwConfig::load(CONFIG_FILE);
    let mut app = Self {
      modes,
      selected,
      live: None,
      check: None,
      c_secs: 5,
      c_target: 1000.0,
      c_err: 500.0,
      c_load: 0.0,
      c_filter: String::new(),
      last_sample: None,
      started_at: Instant::now(),
      msg: format!("就绪。已注册 {} 个测试模式", count),
      smoke,
      unlock_pwd: String::new(),
      unlocked: false,
      rules_emitted: false,
      auto_closed: false,
      rules: RulesGui::new(),
      config: config.clone(),
    };
    // 配置生效：默认秒数/负载/check 参数、规则文件自动加载
    // gui 段：Check 默认参数 / 全局负载
    if config.gui.run_seconds > 0 {
      app.c_secs = config.gui.run_seconds as usize;
    }
    if config.gui.raise_load_percent > 0.0 {
      app.c_load = config.gui.raise_load_percent;
    }
    if config.gui.check_params.secs > 0 {
      app.c_secs = config.gui.check_params.secs;
    }
    app.c_target = config.gui.check_params.target;
    app.c_err = config.gui.check_params.error;
    app.c_load = config.gui.check_params.load;
    app.rules.global_load = config.gui.raise_load_percent;
    app.rules.global_test_mode = config.test_mode;
    // plan 段：统一配置内嵌的测试规则
    if !config.plan.rules.is_empty() {
      app.rules.file = Some(config.plan.clone());
      app.rules.plan = config.plan.name.clone().unwrap_or_else(|| CONFIG_FILE.into());
    }
    if created {
      app.msg = format!("已生成默认配置 {}（etest 可直接修改）", CONFIG_FILE);
    }
    app
  }

  /// 配置是否锁定（锁定后所有配置项只读）
  fn config_locked(&self) -> bool {
    self.config.lock.enabled && !self.unlocked
  }

  /// 解锁：密码正确则本会话内允许编辑
  fn try_unlock(&mut self) {
    if self.unlock_pwd == self.config.lock.password {
      self.unlocked = true;
      self.msg = "已解锁，配置可编辑（仅本会话）".into();
    } else {
      self.msg = "解锁失败：密码错误".into();
    }
  }

  fn start_live(&mut self) {
    self.stop_live();
    let mode = match get_mode(&self.selected) {
      Some(m) => m,
      None => {
        self.msg = format!("未知模式 {}", self.selected);
        return;
      }
    };
    match mode.create() {
      Ok(mut inst) => {
        if let Err(e) = inst.setup(&ModeContext::default()) {
          self.msg = format!("{} 初始化失败: {}", self.selected, e);
          return;
        }
        self.live = Some(LiveSampler {
          mode_name: self.selected.clone(),
          inst,
          t0: Instant::now(),
          series: HashMap::new(),
          last_metrics: Vec::new(),
          error: None,
        });
        self.msg = format!("实时监控已启动: {}", self.selected);
      }
      Err(e) => self.msg = format!("{} 启动失败: {}", self.selected, e),
    }
  }

  fn stop_live(&mut self) {
    if let Some(mut live) = self.live.take() {
      live.stop();
      self.msg = format!("实时监控已停止: {}", live.mode_name);
    }
  }

  fn start_check(&mut self) {
    // 停止旧的 check（未完成的计入历史并标记为停止）
    self.stop_check();
    let mode = match get_mode(&self.selected) {
      Some(m) => m,
      None => {
        self.msg = format!("未知模式 {}", self.selected);
        return;
      }
    };
    let params = TestParams {
      test_secs: self.c_secs.max(1),
      v1: self.c_target,
      v2: self.c_err.abs(),
      v3: self.c_load,
    };
    let filter: Vec<String> = self.c_filter.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    match mode.create() {
      Ok(mut inst) => {
        if let Err(e) = inst.setup(&ModeContext::default()) {
          self.msg = format!("{} 初始化失败: {}", self.selected, e);
          return;
        }
        let ctx = ModeContext::default();
        let load_handles = if params.v3 > 0.0 {
          inst.spawn_load(&ctx, params.v3).unwrap_or_default()
        } else {
          Vec::new()
        };
        self.check = Some(CheckRun {
          mode_name: self.selected.clone(),
          params,
          filter,
          inst,
          t0: Instant::now(),
          samples: Vec::new(),
          sums: HashMap::new(),
          sum_sqs: HashMap::new(),
          stats: HashMap::new(),
          consec: HashMap::new(),
          phase: CheckPhase::Running,
          load_handles,
          error: None,
        });
        self.msg = format!(
          "check 已开始: {} ({} 秒, 目标 {:.1} ±{:.1}, 负载 {:.0}%)",
          self.selected, params.test_secs, params.v1, params.v2, params.v3
        );
      }
      Err(e) => self.msg = format!("{} 启动失败: {}", self.selected, e),
    }
  }

  fn stop_check(&mut self) {
    if let Some(mut check) = self.check.take() {
      if check.phase == CheckPhase::Running {
        check.error = Some("手动停止".into());
        check.finish();
      }
      self.check = Some(check); // 保留结果供查看
    }
  }

  /// 产测报告 JSON（无 R<...>R 包装）
  fn rules_report_json(&self) -> String {
    let report = rules::RulesReport {
      plan: self.rules.plan.clone(),
      status: self.rules.overall_ok(),
      results: self.rules.results.clone(),
    };
    report.to_json().unwrap_or_default()
  }
}
fn install_cjk_fonts(ctx: &egui::Context) {
  use egui::{FontData, FontDefinitions, FontFamily};
  let mut fonts = FontDefinitions::default();
  let candidates = [
    ("C:\\Windows\\Fonts\\msyh.ttc", "msyh"),
    ("C:\\Windows\\Fonts\\msyhbd.ttc", "msyhbd"),
    ("C:\\Windows\\Fonts\\simhei.ttf", "simhei"),
    ("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc", "notocjk"),
    ("/usr/share/fonts/truetype/wqy/wqy-microhei.ttc", "wqy"),
  ];
  let mut found = None;
  for (path, name) in candidates {
    if let Ok(bytes) = std::fs::read(path) {
      fonts.font_data.insert(name.to_string(), Arc::new(FontData::from_owned(bytes)));
      found = Some(name.to_string());
      break;
    }
  }
  if let Some(name) = found {
    fonts.families.entry(FontFamily::Proportional).or_default().push(name.clone());
    fonts.families.entry(FontFamily::Monospace).or_default().push(name);
    ctx.set_fonts(fonts);
  }
}

impl eframe::App for GuiApp {
  fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
    let ctx = ui.ctx().clone();

    // 采样节拍：主线程每 1 秒采样一次（实例不跨线程，规避 !Send 的 WMI 后端）
    let now = Instant::now();
    if self.last_sample.map_or(true, |t| now.duration_since(t) >= SAMPLE_INTERVAL) {
      self.last_sample = Some(now);
      if let Some(live) = &mut self.live {
        live.step();
      }
      if let Some(check) = &mut self.check {
        check.step();
        if check.phase == CheckPhase::Done {
          let ok = check.status_ok();
          let mode = check.mode_name.clone();
          let err = check.error.clone().unwrap_or_default();
          let status_txt = if ok { "PASS" } else { "FAIL" };
          self.msg = format!(
            "check 完成: {} -> {}{}",
            mode,
            status_txt,
            if err.is_empty() { String::new() } else { format!(" ({})", err) }
          );
        }
      }
      // etest 规则逐条执行
      if let Some(run) = &mut self.rules.run {
        if run.step() {
          let res = run.result();
          self.rules.results.push(res);
          self.rules.run = None;
          self.rules.advance();
        }
      }
      // 自动运行（配置 auto_run 或冒烟模式）
      if (self.config.gui.auto_run || self.smoke)
        && self.rules.file.is_some()
        && !self.rules.done
        && self.rules.run.is_none()
        && self.rules.results.is_empty()
      {
        self.rules.start();
      }
      // 总时长上限
      if let Some(started) = self.rules.run_started {
        if self.config.gui.run_seconds > 0 && started.elapsed().as_secs() >= self.config.gui.run_seconds {
          self.rules.timeout("超过总时长上限");
        }
      }
    }
    // 规则完成：报告 JSON 走 e-log 明细；R<...>R 结果作为结果日志的最后一行追加
    if self.rules.done && !self.rules.results.is_empty() && !self.rules_emitted {
      self.rules_emitted = true;
      let json = self.rules_report_json();
      hw::p(&json); // e-log 明细（logs/hw-*.log + stderr）
      hw::write_result_line(&hw::rr_line(&json, self.rules.overall_ok()));
      if self.config.gui.auto_close {
        self.auto_closed = true;
        let ok = self.rules.overall_ok();
        let code = if !ok && self.config.gui.exit_code_on_fail { 1 } else { 0 };
        EXIT_CODE.store(code, Ordering::SeqCst);
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
      }
    }
    ctx.request_repaint_after(SAMPLE_INTERVAL);

    // 冒烟兜底：超时强制关闭
    if self.smoke && self.started_at.elapsed() > Duration::from_secs(6) {
      ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    // ---------------- 顶部控制栏 ----------------
    egui::Panel::top("controls").show(ui, |ui| {
      ui.add_space(4.0);
      ui.horizontal(|ui| {
        ui.heading("HW Monitor GUI");
        ui.separator();

        ui.label("模式:");
        egui::ComboBox::from_id_salt("mode_sel")
          .width(300.0)
          .selected_text(self.selected.clone())
          .show_ui(ui, |ui| {
            for (name, desc) in &self.modes {
              ui.selectable_value(&mut self.selected, name.clone(), format!("{} — {}", name, desc));
            }
          });
        let live_running = self.live.is_some();
        if ui.add_enabled(!live_running, egui::Button::new("▶ 实时监控")).clicked() {
          self.start_live();
        }
        if ui.add_enabled(live_running, egui::Button::new("⏹ 停止")).clicked() {
          self.stop_live();
        }
      });
      ui.add_space(4.0);
      let locked = self.config_locked();
      ui.horizontal(|ui| {
        ui.label("秒数");
        ui.add_enabled(!locked, egui::DragValue::new(&mut self.c_secs).range(1..=3600));
        ui.label("目标");
        ui.add_enabled(!locked, egui::DragValue::new(&mut self.c_target).speed(1.0));
        ui.label("±误差");
        ui.add_enabled(!locked, egui::DragValue::new(&mut self.c_err).speed(1.0));
        ui.label("负载%");
        ui.add_enabled(!locked, egui::DragValue::new(&mut self.c_load).speed(1.0).range(0.0..=100.0));
        ui.label("校验过滤(逗号分隔,空=全部)");
        ui.add_enabled(!locked, egui::TextEdit::singleline(&mut self.c_filter).desired_width(160.0));
        let check_running = self.check.as_ref().map_or(false, |c| c.phase == CheckPhase::Running);
        if ui.add_enabled(!check_running, egui::Button::new("▶ Check")).clicked() {
          self.start_check();
        }
        if ui.add_enabled(check_running, egui::Button::new("⏹ 停止")).clicked() {
          self.stop_check();
        }
        ui.separator();
        // etest 规则运行
        let rules_running = self.rules.run.is_some();
        let rules_can_run = self.rules.file.is_some() && !rules_running && !self.rules.done;
        if ui.add_enabled(rules_can_run, egui::Button::new("▶ 运行规则")).clicked() {
          self.rules.start();
        }
        if ui.add_enabled(rules_running, egui::Button::new("⏹ 停止")).clicked() {
          self.rules.stop();
        }
        ui.separator();
        if ui.button("导出报告").clicked() {
          self.rules.export_report();
        }
        if ui.button("清空结果").clicked() {
          self.rules.results.clear();
          self.rules.run = None;
          self.rules.done = false;
        }
      });
      // 配置锁状态
      if self.config.lock.enabled {
        ui.add_space(2.0);
        ui.horizontal(|ui| {
          if self.unlocked {
            ui.colored_label(egui::Color32::from_rgb(120, 200, 120), "🔓 已解锁（本会话）");
          } else {
            ui.colored_label(egui::Color32::from_rgb(230, 120, 120), "🔒 配置已锁定（只读，防误改）");
            ui.label("解锁密码:");
            ui.add(egui::TextEdit::singleline(&mut self.unlock_pwd).password(true).desired_width(120.0));
            if ui.button("解锁").clicked() {
              self.try_unlock();
            }
          }
        });
      }
      if !self.msg.is_empty() {
        ui.add_space(2.0);
        ui.colored_label(egui::Color32::from_rgb(255, 220, 120), &self.msg);
      }
      ui.add_space(4.0);
    });

    // ---------------- 左侧：测试功能项 + 实时指标 ----------------
    egui::Panel::left("left_panel").default_size(380.0).show(ui, |ui| {
      ui.add_space(4.0);
      ui.heading("测试功能项");
      let locked = self.config_locked();
      let mut toggles: Vec<usize> = Vec::new();
      if let Some(file) = &self.rules.file {
        egui::ScrollArea::vertical().max_height(340.0).show(ui, |ui| {
          for (i, rule) in file.rules.iter().enumerate() {
            ui.horizontal(|ui| {
              let mut en = rule.enabled;
              if ui.add_enabled(!locked, egui::Checkbox::new(&mut en, "")).changed() && en != rule.enabled {
                toggles.push(i);
              }
              // 上次结果状态
              let st = self.rules.results.iter().find(|res| res.item == rule.id);
              if let Some(res) = st {
                if res.skipped {
                  ui.colored_label(egui::Color32::from_rgb(150, 150, 150), "跳过");
                } else if res.pass {
                  ui.colored_label(egui::Color32::from_rgb(120, 200, 120), "PASS");
                } else {
                  ui.colored_label(egui::Color32::from_rgb(230, 120, 120), "FAIL");
                }
              } else {
                ui.weak("-");
              }
              ui.label(if rule.description.is_empty() {
                rule.id.clone()
              } else {
                rule.description.clone()
              });
            });
          }
        });
        if locked {
          ui.weak("🔒 已锁定：勾选是否测试需先解锁");
        }
      } else {
        ui.weak("（未加载规则，启动自动读取 hw-config.json 的 plan）");
      }
      for i in toggles {
        self.toggle_rule_enabled(i);
      }

      ui.add_space(10.0);
      ui.heading("实时指标");
      if let Some(live) = &self.live {
        if let Some(err) = &live.error {
          ui.colored_label(egui::Color32::RED, err);
        }
        egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
          egui::Grid::new("live_table").striped(true).num_columns(3).show(ui, |ui| {
            ui.strong("指标");
            ui.strong("值");
            ui.strong("单位");
            ui.end_row();
            for m in live.last_metrics.iter().filter(|m| self.config.gui.metric_visible(&m.name)) {
              ui.label(&m.name);
              ui.label(format!("{:.2}", m.value));
              ui.label(&m.unit);
              ui.end_row();
            }
          });
        });
      } else {
        ui.weak("（未启动实时监控）");
      }
    });

    // ---------------- 中央：测试数据线图（固定布局，测试时更新） ----------------
    egui::CentralPanel::default().show(ui, |ui| {
      ui.add_space(4.0);
      // 状态头
      let header = if let Some(run) = &self.rules.run {
        format!(
          "规则 {}/{}：{}（{}）",
          self.rules.results.len() + 1,
          self.rules.total(),
          run.rule.id,
          run.rule.description
        )
      } else if let Some(check) = &self.check {
        format!("Check 测试：{}（{} 秒）", check.mode_name, check.params.test_secs)
      } else if let Some(live) = &self.live {
        format!("实时监控：{}", live.mode_name)
      } else {
        "空闲".to_string()
      };
      ui.heading(format!("测试数据线图 — {}", header));

      // 组装曲线数据源（优先当前规则样本 → 实时 → Check）
      let mut series: Vec<(String, Vec<(f64, f64)>)> = Vec::new();
      let mut band: Option<(f64, f64)> = None;
      if let Some(run) = &self.rules.run {
        let mut names: Vec<String> = run.samples.iter().flat_map(|(_, ms)| ms.iter().map(|m| m.name.clone())).collect();
        names.sort_unstable();
        names.dedup();
        for name in names {
          let pts: Vec<(f64, f64)> = run
            .samples
            .iter()
            .filter_map(|(t, ms)| ms.iter().find(|m| m.name == name).map(|m| (*t, m.value)))
            .collect();
          series.push((name, pts));
        }
      } else if let Some(live) = &self.live {
        let mut names: Vec<String> = live.series.keys().filter(|n| self.config.gui.metric_visible(n)).cloned().collect();
        names.sort_unstable();
        for name in names {
          if let Some(pts) = live.series.get(&name) {
            series.push((name, pts.clone()));
          }
        }
      } else if let Some(check) = &self.check {
        band = Some((check.params.range_min(), check.params.range_max()));
        let mut names: Vec<String> = check.stats.keys().cloned().collect();
        names.sort_unstable();
        for name in names {
          let pts: Vec<(f64, f64)> = check
            .samples
            .iter()
            .filter_map(|(t, ms)| ms.iter().find(|m| m.name == name).map(|m| (*t, m.value)))
            .collect();
          series.push((name, pts));
        }
      }
      if series.is_empty() {
        ui.weak("开始测试后，这里固定显示测试数据线图（每项一条曲线，测试时实时更新）。");
      } else {
        self.plot_grid(ui, &series, band);
      }
    });
  }
}

impl GuiApp {
  /// 切换某个功能项是否测试（enabled），并保存到 hw-config.json（需解锁）
  fn toggle_rule_enabled(&mut self, idx: usize) {
    if self.config_locked() {
      self.msg = "配置已锁定，请先解锁".into();
      return;
    }
    let mut changed = false;
    if let Some(file) = self.rules.file.as_mut() {
      if let Some(rule) = file.rules.get_mut(idx) {
        rule.enabled = !rule.enabled;
        changed = true;
      }
    }
    if changed {
      if let Some(file) = &self.rules.file {
        self.config.plan = file.clone();
      }
      match self.config.save(CONFIG_FILE) {
        Ok(_) => self.msg = "已更新是否测试并保存到 hw-config.json".into(),
        Err(e) => self.msg = format!("保存配置失败: {}", e),
      }
    }
  }

  /// 中央固定线图网格：每个指标一个小图（2 列），测试时数据实时更新
  fn plot_grid(&self, ui: &mut egui::Ui, series: &[(String, Vec<(f64, f64)>)], band: Option<(f64, f64)>) {
    let band_color = egui::Color32::from_rgb(90, 160, 220);
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
      ui.columns(2, |cols| {
        for (idx, (name, pts)) in series.iter().enumerate() {
          let ui = &mut cols[idx % 2];
          if pts.is_empty() {
            continue;
          }
          let points: Vec<[f64; 2]> = pts.iter().map(|&(x, y)| [x, y]).collect();
          let line = Line::new(name.clone(), PlotPoints::from(points)).width(2.0);
          Plot::new(format!("chart_{}", name)).height(130.0).legend(Legend::default()).show(ui, |pui| {
            pui.line(line);
            if let Some((lo, hi)) = band {
              pui.hline(HLine::new("下限", lo).color(band_color));
              pui.hline(HLine::new("上限", hi).color(band_color));
            }
          });
        }
      });
    });
  }
}
