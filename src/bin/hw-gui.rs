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

use hw::gui_config::{GuiConfig, CONFIG_FILE};
use hw::test_mode::{
  get as get_mode, list as list_modes, register_all, rules, Metric, MetricStat, ModeContext, ModeInstance,
  TestParams,
};

const SAMPLE_INTERVAL: Duration = Duration::from_secs(1);
const MAX_POINTS: usize = 900;
const HISTORY_FILE: &str = "hw-gui-history.json";

/// 进程退出码（auto_close 时由测试结果决定，供 etest 判断）
static EXIT_CODE: AtomicI32 = AtomicI32::new(0);

fn main() -> eframe::Result {
  register_all();
  let smoke = std::env::var("HW_GUI_SMOKE").is_ok();
  let opts = eframe::NativeOptions {
    viewport: egui::ViewportBuilder::default()
      .with_inner_size([1280.0, 840.0])
      .with_min_inner_size([960.0, 600.0])
      .with_title("HW Monitor GUI"),
    ..Default::default()
  };
  let result = eframe::run_native(
    "HW Monitor GUI",
    opts,
    Box::new(|cc| Ok(Box::new(GuiApp::new(cc, smoke)))),
  );
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

#[derive(Clone, Copy, PartialEq)]
enum View {
  Live,
  Check,
  Rules,
}

/// etest 规则执行面板状态（与 CLI run-rules 共用 rules::RuleRun）
struct RulesGui {
  path: String,
  file: Option<rules::RuleFile>,
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
      path: "etest-rules.json".into(),
      file: None,
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

  fn load(&mut self) {
    match rules::parse_rules(&self.path) {
      Ok(file) => {
        self.plan = file.name.clone().unwrap_or_else(|| self.path.clone());
        self.results.clear();
        self.run = None;
        self.done = false;
        self.msg = format!("已加载规则文件：{}（{} 条）", self.plan, file.rules.len());
        self.file = Some(file);
      }
      Err(e) => {
        self.file = None;
        self.msg = format!("加载失败: {}", e);
      }
    }
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

  /// 推进到下一条规则；全部完成时置 done
  fn advance(&mut self) {
    let total = self.total();
    if self.results.len() >= total {
      let ok = self.results.iter().all(|r| r.pass);
      let plan = self.plan.clone();
      self.msg = format!("{} 完成：{} / {}", plan, if ok { "PASS" } else { "FAIL" }, self.results.len());
      self.done = true;
      return;
    }
    let idx = self.results.len();
    let mut rule = self.file.as_ref().unwrap().rules[idx].clone();
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
      self.results.push(rules::RuleResult::failed(&rule, "未执行（已停止）"));
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
      self.results.push(rules::RuleResult::failed(&rule, reason));
    }
    self.done = true;
    self.run_started = None;
    self.msg = reason.into();
  }

  fn export_report(&mut self) {
    let path = format!("etest-report-{}.json", now_str());
    let report = rules::RulesReport {
      plan: self.plan.clone(),
      status: self.results.iter().all(|r| r.pass),
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
  collected: bool,
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
      st.finish(
        self.sums.get(name).copied().unwrap_or(0.0),
        self.sum_sqs.get(name).copied().unwrap_or(0.0),
      );
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

// ---------------------------------------------------------------------------
// 历史记录
// ---------------------------------------------------------------------------
#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct HistoryEntry {
  time: String,
  mode: String,
  params: TestParams,
  status: bool,
  error: Option<String>,
  metrics: Vec<MetricStat>,
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
  history: Vec<HistoryEntry>,
  last_sample: Option<Instant>,
  started_at: Instant,
  msg: String,
  smoke: bool,
  rules_emitted: bool,
  auto_closed: bool,
  view: View,
  rules: RulesGui,
  config: GuiConfig,
}

impl GuiApp {
  fn new(cc: &eframe::CreationContext<'_>, smoke: bool) -> Self {
    install_cjk_fonts(&cc.egui_ctx);
    let modes: Vec<(String, String)> = list_modes()
      .into_iter()
      .map(|(n, d)| (n.to_string(), d.to_string()))
      .collect();
    let selected = modes
      .first()
      .map(|(n, _)| n.clone())
      .unwrap_or_else(|| "net-speed".into());
    let history = load_history();
    let count = modes.len();
    // 加载运行配置（缺失自动创建模板）
    let (config, created) = GuiConfig::load(CONFIG_FILE);
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
      history,
      last_sample: None,
      started_at: Instant::now(),
      msg: format!("就绪。已注册 {} 个测试模式", count),
      smoke,
      rules_emitted: false,
      auto_closed: false,
      view: match config.view() {
        "live" => View::Live,
        "check" => View::Check,
        _ => View::Rules,
      },
      rules: RulesGui::new(),
      config: config.clone(),
    };
    // 配置生效：默认秒数/负载/check 参数、规则文件自动加载
    if config.run_seconds > 0 {
      app.c_secs = config.run_seconds as usize;
    }
    if config.raise_load_percent > 0.0 {
      app.c_load = config.raise_load_percent;
    }
    if config.check_params.secs > 0 {
      app.c_secs = config.check_params.secs;
    }
    app.c_target = config.check_params.target;
    app.c_err = config.check_params.error;
    app.c_load = config.check_params.load;
    app.rules.path = config.rule_file.clone();
    app.rules.global_load = config.raise_load_percent;
    if std::path::Path::new(&config.rule_file).exists() {
      app.rules.load();
    }
    if created {
      app.msg = format!("已生成默认配置 {}（etest 可直接修改）", CONFIG_FILE);
    }
    app
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
    let filter: Vec<String> = self
      .c_filter
      .split(',')
      .map(|s| s.trim().to_string())
      .filter(|s| !s.is_empty())
      .collect();
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
          collected: false,
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
      self.collect_history(&mut check);
    }
  }

  /// 将已完成的 check 写入历史并持久化
  fn collect_history(&mut self, check: &mut CheckRun) {
    if check.collected {
      return;
    }
    check.collected = true;
    let mut metrics: Vec<MetricStat> = check.stats.values().cloned().collect();
    metrics.sort_by(|a, b| a.name.cmp(&b.name));
    self.history.push(HistoryEntry {
      time: now_str(),
      mode: check.mode_name.clone(),
      params: check.params,
      status: check.status_ok(),
      error: check.error.clone(),
      metrics,
    });
    if self.history.len() > 200 {
      self.history.remove(0);
    }
    self.save_history();
  }

  /// 构造 etest 兼容的 R<...>R 结果行（与 CLI main.rs 的 CmdResult 结构完全一致）
  fn rules_etest_line(&self) -> String {
    use e_utils::cmd::CmdResult;
    let report = rules::RulesReport {
      plan: self.rules.plan.clone(),
      status: self.rules.results.iter().all(|r| r.pass),
      results: self.rules.results.clone(),
    };
    let content = report.to_json().unwrap_or_default();
    let res: CmdResult<serde_json::Value> = CmdResult {
      content,
      status: report.status,
      opts: serde_json::Value::Null,
    };
    res.to_str().unwrap_or_default()
  }

  fn save_history(&self) {
    let json = serde_json::to_string_pretty(&self.history).unwrap_or_default();
    let _ = std::fs::write(HISTORY_FILE, json);
  }

  fn export_json(&mut self) {
    let path = format!("hw-gui-history-{}.json", now_str());
    let json = serde_json::to_string_pretty(&self.history).unwrap_or_default();
    match std::fs::write(&path, json) {
      Ok(_) => self.msg = format!("已导出 {}", path),
      Err(e) => self.msg = format!("导出失败: {}", e),
    }
  }

  fn export_csv(&mut self) {
    let path = format!("hw-gui-history-{}.csv", now_str());
    let mut csv = String::from("time,mode,secs,target,err,load,status,metric,unit,value,min,max,avg,std_dev,samples,pass\n");
    for h in &self.history {
      let status = if h.status { "PASS" } else { "FAIL" };
      for m in &h.metrics {
        csv.push_str(&format!(
          "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
          h.time,
          h.mode,
          h.params.test_secs,
          h.params.v1,
          h.params.v2,
          h.params.v3,
          status,
          m.name,
          m.unit,
          m.value,
          m.min,
          m.max,
          m.avg,
          m.std_dev,
          m.samples,
          m.pass.map(|p| if p { "PASS" } else { "FAIL" }).unwrap_or(""),
        ));
      }
    }
    match std::fs::write(&path, csv) {
      Ok(_) => self.msg = format!("已导出 {}", path),
      Err(e) => self.msg = format!("导出失败: {}", e),
    }
  }
}

fn load_history() -> Vec<HistoryEntry> {
  std::fs::read_to_string(HISTORY_FILE)
    .ok()
    .and_then(|s| serde_json::from_str(&s).ok())
    .unwrap_or_default()
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
    fonts
      .families
      .entry(FontFamily::Proportional)
      .or_default()
      .push(name.clone());
    fonts.families.entry(FontFamily::Monospace).or_default().push(name);
    ctx.set_fonts(fonts);
  }
}

impl eframe::App for GuiApp {
  fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
    let ctx = ui.ctx().clone();

    // 采样节拍：主线程每 1 秒采样一次（实例不跨线程，规避 !Send 的 WMI 后端）
    let now = Instant::now();
    if self
      .last_sample
      .map_or(true, |t| now.duration_since(t) >= SAMPLE_INTERVAL)
    {
      self.last_sample = Some(now);
      if let Some(live) = &mut self.live {
        live.step();
      }
      if let Some(check) = &mut self.check {
        check.step();
        if check.phase == CheckPhase::Done {
          let mut done = self.check.take().unwrap();
          self.collect_history(&mut done);
          let ok = done.status_ok();
          let mode = done.mode_name.clone();
          let err = done.error.clone().unwrap_or_default();
          let status_txt = if ok { "PASS" } else { "FAIL" };
          self.msg = format!(
            "check 完成: {} -> {}{}",
            mode,
            status_txt,
            if err.is_empty() {
              String::new()
            } else {
              format!(" ({})", err)
            }
          );
          self.check = Some(done); // 保留结果供查看
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
      if (self.config.auto_run || self.smoke)
        && self.rules.file.is_some()
        && !self.rules.done
        && self.rules.run.is_none()
        && self.rules.results.is_empty()
      {
        self.rules.start();
      }
      // 总时长上限
      if let Some(started) = self.rules.run_started {
        if self.config.run_seconds > 0 && started.elapsed().as_secs() >= self.config.run_seconds {
          self.rules.timeout("超过总时长上限");
        }
      }
    }
    // 规则完成：输出 R<...>R 结果（stdout + 日志文件），与 CLI 协议一致，etest 统一解析
    if self.rules.done && !self.rules.results.is_empty() && !self.rules_emitted {
      self.rules_emitted = true;
      let line = self.rules_etest_line();
      println!("{}", line);
      if !self.config.log_file.is_empty() {
        let ts = now_str();
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&self.config.log_file) {
          use std::io::Write;
          let _ = f.write_all(format!("{}	{}
", ts, line).as_bytes());
        }
      }
      if self.config.auto_close {
        self.auto_closed = true;
        let ok = self.rules.results.iter().all(|r| r.pass);
        let code = if !ok && self.config.exit_code_on_fail { 1 } else { 0 };
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
        ui.selectable_value(&mut self.view, View::Live, "实时监控");
        ui.selectable_value(&mut self.view, View::Check, "Check 测试");
        ui.selectable_value(&mut self.view, View::Rules, "etest 规则");
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
        if ui
          .add_enabled(!live_running, egui::Button::new("▶ 实时监控"))
          .clicked()
        {
          self.start_live();
        }
        if ui
          .add_enabled(live_running, egui::Button::new("⏹ 停止"))
          .clicked()
        {
          self.stop_live();
        }
      });
      ui.add_space(4.0);
      ui.horizontal(|ui| {
        ui.label("秒数");
        ui.add(egui::DragValue::new(&mut self.c_secs).range(1..=3600));
        ui.label("目标");
        ui.add(egui::DragValue::new(&mut self.c_target).speed(1.0));
        ui.label("±误差");
        ui.add(egui::DragValue::new(&mut self.c_err).speed(1.0));
        ui.label("负载%");
        ui.add(egui::DragValue::new(&mut self.c_load).speed(1.0).range(0.0..=100.0));
        ui.label("校验过滤(逗号分隔,空=全部)");
        ui.add(egui::TextEdit::singleline(&mut self.c_filter).desired_width(160.0));
        let check_running = self
          .check
          .as_ref()
          .map_or(false, |c| c.phase == CheckPhase::Running);
        if ui
          .add_enabled(!check_running, egui::Button::new("▶ Check"))
          .clicked()
        {
          self.start_check();
        }
        if ui
          .add_enabled(check_running, egui::Button::new("⏹ 停止"))
          .clicked()
        {
          self.stop_check();
        }
        ui.separator();
        if ui.button("导出 JSON").clicked() {
          self.export_json();
        }
        if ui.button("导出 CSV").clicked() {
          self.export_csv();
        }
        if ui.button("清空历史").clicked() {
          self.history.clear();
          self.save_history();
        }
      });
      if !self.msg.is_empty() {
        ui.add_space(2.0);
        ui.colored_label(egui::Color32::from_rgb(255, 220, 120), &self.msg);
      }
      ui.add_space(4.0);
    });

    // ---------------- 左侧：实时指标表 + 历史 ----------------
    egui::Panel::left("left_panel")
      .default_size(360.0)
      .show(ui, |ui| {
        ui.add_space(4.0);
        ui.heading("实时指标");
        if let Some(live) = &self.live {
          if let Some(err) = &live.error {
            ui.colored_label(egui::Color32::RED, err);
          }
          egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
            egui::Grid::new("live_table")
              .striped(true)
              .num_columns(3)
              .show(ui, |ui| {
                ui.strong("指标");
                ui.strong("值");
                ui.strong("单位");
                ui.end_row();
                for m in live.last_metrics.iter().filter(|m| self.config.metric_visible(&m.name)) {
                  ui.label(&m.name);
                  ui.label(format!("{:.2}", m.value));
                  ui.label(&m.unit);
                  ui.end_row();
                }
              });
          });
        } else {
          ui.weak("（未启动，选择模式后点击 ▶ 实时监控）");
        }

        ui.add_space(12.0);
        ui.heading("测试历史");
        egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
          for (i, h) in self.history.iter().enumerate().rev() {
            let color = if h.status {
              egui::Color32::from_rgb(120, 200, 120)
            } else {
              egui::Color32::from_rgb(230, 120, 120)
            };
            ui.horizontal(|ui| {
              ui.colored_label(color, if h.status { "PASS" } else { "FAIL" });
              ui.label(format!("{} {}", h.mode, h.time));
            });
            ui.weak(format!(
              "  秒数{} 目标{:.0} ±{:.0} 负载{:.0}% 指标{}",
              h.params.test_secs,
              h.params.v1,
              h.params.v2,
              h.params.v3,
              h.metrics.len()
            ));
            if let Some(err) = &h.error {
              ui.weak(format!("  {}", err));
            }
            if i % 8 == 0 {
              ui.add_space(2.0);
            }
          }
        });
      });

    // ---------------- 中央：实时曲线 / check 曲线 ----------------
    egui::CentralPanel::default().show(ui, |ui| {
      ui.add_space(4.0);
      match self.view {
        View::Live => {
          if let Some(live) = &self.live {
            ui.heading(format!("实时曲线 — {}", live.mode_name));
            let names: Vec<String> = live
              .series
              .keys()
              .filter(|n| self.config.metric_visible(n))
              .cloned()
              .collect();
            if names.is_empty() {
              ui.weak("等待首个样本…");
            } else {
              egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                  ui.columns(2, |cols| {
                    for (idx, name) in names.iter().enumerate() {
                      let ui = &mut cols[idx % 2];
                      if let Some(points) = live.series.get(name) {
                        let pts: Vec<[f64; 2]> = points.iter().map(|&(x, y)| [x, y]).collect();
                        let line = Line::new(name.clone(), PlotPoints::from(pts)).width(2.0);
                        Plot::new(format!("live_{}", name))
                          .height(110.0)
                          .legend(Legend::default())
                          .show(ui, |pui| {
                            pui.line(line);
                          });
                      }
                    }
                  });
                });
            }
          } else {
            ui.weak("选择模式后点击「▶ 实时监控」查看实时曲线。");
          }
        }
        View::Check => {
          if let Some(check) = &self.check {
            self.show_check_panel(ui, check);
          } else {
            ui.weak("设置参数后点击「▶ Check」运行测试。");
          }
        }
        View::Rules => self.show_rules_panel(ui),
      }
    });
  }
}

impl GuiApp {
  fn show_check_panel(&self, ui: &mut egui::Ui, check: &CheckRun) {
    let running = check.phase == CheckPhase::Running;
    ui.heading(format!(
      "Check — {} ({}{})",
      check.mode_name,
      if running { "运行中 " } else { "" },
      if running {
        format!("{}/{} 秒", check.samples.len(), check.params.test_secs)
      } else {
        "完成".into()
      }
    ));
    let color = if check.status_ok() {
      egui::Color32::from_rgb(120, 200, 120)
    } else {
      egui::Color32::from_rgb(230, 120, 120)
    };
    ui.colored_label(color, if check.status_ok() { "PASS" } else { "FAIL" });
    if let Some(err) = &check.error {
      ui.colored_label(egui::Color32::YELLOW, err);
    }

    // 曲线：每个指标一个迷你图，叠加目标带
    let names: Vec<String> = check.stats.keys().cloned().collect();
    if names.is_empty() {
      ui.weak("等待首个样本…");
      return;
    }
    egui::ScrollArea::vertical()
      .auto_shrink([false, false])
      .show(ui, |ui| {
        ui.columns(2, |cols| {
          for (idx, name) in names.iter().enumerate() {
            let ui = &mut cols[idx % 2];
            let pts: Vec<[f64; 2]> = check
              .samples
              .iter()
              .filter_map(|(t, ms)| ms.iter().find(|m| &m.name == name).map(|m| [*t, m.value]))
              .collect();
            if pts.is_empty() {
              continue;
            }
            let line = Line::new(name.clone(), PlotPoints::from(pts)).width(2.0);
            let y_min = check.params.range_min();
            let y_max = check.params.range_max();
            let band_color = egui::Color32::from_rgb(90, 160, 220);
            Plot::new(format!("check_{}", name))
              .height(130.0)
              .legend(Legend::default())
              .show(ui, |pui| {
                pui.line(line);
                pui.hline(HLine::new("下限", y_min).color(band_color));
                pui.hline(HLine::new("上限", y_max).color(band_color));
              });
          }
        });
      });

    // 结果统计表
    let mut metrics: Vec<&MetricStat> = check.stats.values().collect();
    metrics.sort_by(|a, b| a.name.cmp(&b.name));
    ui.add_space(6.0);
    egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
      egui::Grid::new("check_stats")
        .striped(true)
        .num_columns(9)
        .show(ui, |ui| {
          for h in ["指标", "最后", "最小", "最大", "平均", "标准差", "样本", "单位", "判定"] {
            ui.strong(h);
          }
          ui.end_row();
          for m in metrics {
            ui.label(&m.name);
            ui.label(format!("{:.2}", m.value));
            ui.label(format!("{:.2}", m.min));
            ui.label(format!("{:.2}", m.max));
            ui.label(format!("{:.2}", m.avg));
            ui.label(format!("{:.2}", m.std_dev));
            ui.label(m.samples.to_string());
            ui.label(&m.unit);
            match m.pass {
              Some(true) => {
                ui.colored_label(egui::Color32::from_rgb(120, 200, 120), "PASS");
              }
              Some(false) => {
                ui.colored_label(egui::Color32::from_rgb(230, 120, 120), "FAIL");
              }
              None => {
                ui.weak("-");
              }
            }
            ui.end_row();
          }
        });
    });
  }

  /// etest 规则执行面板
  fn show_rules_panel(&mut self, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
      ui.label("规则文件:");
      ui.add(egui::TextEdit::singleline(&mut self.rules.path).desired_width(260.0));
      if ui.button("加载").clicked() {
        self.rules.load();
      }
      let can_run = self.rules.file.is_some() && self.rules.run.is_none() && !self.rules.done;
      if ui
        .add_enabled(can_run, egui::Button::new("▶ 运行规则"))
        .clicked()
      {
        self.rules.start();
      }
      let running = self.rules.run.is_some();
      if ui.add_enabled(running, egui::Button::new("⏹ 停止")).clicked() {
        self.rules.stop();
      }
      if ui.button("导出报告").clicked() {
        self.rules.export_report();
      }
      if ui.button("清空结果").clicked() {
        self.rules.results.clear();
        self.rules.run = None;
        self.rules.done = false;
      }
    });
    if !self.rules.msg.is_empty() {
      ui.colored_label(egui::Color32::from_rgb(255, 220, 120), &self.rules.msg);
    }

    // 规则清单
    if let Some(file) = &self.rules.file {
      ui.add_space(6.0);
      ui.heading(format!("规则清单（{}）", file.rules.len()));
      egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
        egui::Grid::new("rules_table")
          .striped(true)
          .num_columns(8)
          .show(ui, |ui| {
            for h in ["#", "id", "模式", "指标", "下限", "上限", "稳定σ", "秒数", "负载%"] {
              ui.strong(h);
            }
            ui.end_row();
            for (i, r) in file.rules.iter().enumerate() {
              ui.label((i + 1).to_string());
              ui.label(&r.id);
              ui.label(&r.mode);
              ui.label(if r.metric.is_empty() {
                "全部".into()
              } else {
                r.metric.clone()
              });
              ui.label(r.min.map(|v| v.to_string()).unwrap_or_else(|| "-".into()));
              ui.label(r.max.map(|v| v.to_string()).unwrap_or_else(|| "-".into()));
              ui.label(r.max_std.map(|v| v.to_string()).unwrap_or_else(|| "-".into()));
              ui.label(r.secs.to_string());
              ui.label(r.load.to_string());
              ui.end_row();
            }
          });
      });
    }

    // 当前规则进度 + 曲线
    if let Some(run) = &self.rules.run {
      ui.add_space(6.0);
      ui.horizontal(|ui| {
        ui.spinner();
        ui.label(format!(
          "运行中 {}/{}：{}（{}）",
          self.rules.results.len() + 1,
          self.rules.total(),
          run.rule.id,
          run.rule.mode
        ));
      });
      let last: Vec<&Metric> = run
        .last_metrics()
        .iter()
        .filter(|m| self.config.metric_visible(&m.name))
        .collect();
      if !last.is_empty() {
        ui.horizontal_wrapped(|ui| {
          for m in last {
            ui.label(format!("{}={:.2}{}", m.name, m.value, m.unit));
          }
        });
      }
      if let Some(first) = run.samples.first().and_then(|(_, ms)| ms.first().cloned()) {
        let pts: Vec<[f64; 2]> = run
          .samples
          .iter()
          .filter_map(|(t, ms)| {
            ms.iter()
              .find(|m| m.name == first.name)
              .map(|m| [*t, m.value])
          })
          .collect();
        if !pts.is_empty() {
          let line = Line::new(first.name.clone(), PlotPoints::from(pts)).width(2.0);
          Plot::new("rule_cur")
            .height(120.0)
            .legend(Legend::default())
            .show(ui, |pui| {
              pui.line(line);
            });
        }
      }
    }

    // 结果表
    if !self.rules.results.is_empty() {
      ui.add_space(6.0);
      let ok = self.rules.results.iter().all(|r| r.pass);
      let color = if ok {
        egui::Color32::from_rgb(120, 200, 120)
      } else {
        egui::Color32::from_rgb(230, 120, 120)
      };
      ui.colored_label(color, format!("整体: {}", if ok { "PASS" } else { "FAIL" }));
      egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
        egui::Grid::new("rules_results")
          .striped(true)
          .num_columns(9)
          .show(ui, |ui| {
            for h in ["项", "模式", "指标", "平均", "最小", "最大", "限制", "判定", "说明"] {
              ui.strong(h);
            }
            ui.end_row();
            for r in &self.rules.results {
              ui.label(&r.item);
              ui.label(&r.mode);
              ui.label(&r.metric);
              ui.label(format!("{:.2}", r.avg));
              ui.label(format!("{:.2}", r.min));
              ui.label(format!("{:.2}", r.max));
              let mut lim = match (r.min_limit, r.max_limit) {
                (Some(lo), Some(hi)) => format!("{:.0}~{:.0}", lo, hi),
                (Some(lo), None) => format!("≥{:.0}", lo),
                (None, Some(hi)) => format!("≤{:.0}", hi),
                (None, None) => "-".into(),
              };
              if let Some(ms) = r.max_std_limit {
                lim.push_str(&format!(" σ≤{:.1}", ms));
              }
              ui.label(lim);
              if r.pass {
                ui.colored_label(egui::Color32::from_rgb(120, 200, 120), "PASS");
              } else {
                ui.colored_label(egui::Color32::from_rgb(230, 120, 120), "FAIL");
              }
              ui.label(r.message.clone().unwrap_or_default());
              ui.end_row();
            }
          });
      });
    }
  }
}
