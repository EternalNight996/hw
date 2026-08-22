//! # 日志（e-log / tracing）
//!
//! 参考 e-log crate（与 etest 项目同源的日志库）的用法：`init_logging()` 初始化
//! 一个 tracing subscriber —— 按天滚动写日志文件 `logs/hw-*.log`，同时输出到 stderr；
//! stdout 保留给 etest 的 R<...>R 协议。`hw::p/ep/wp/dp` 与 GUI 的运行日志均走此处。

/// 初始化日志（幂等，可重复调用）。需 tracing 特性。
pub fn init_logging() {
  #[cfg(feature = "tracing")]
  {
    use e_log::subscriber::layer::SubscriberExt as _;
    let folder = std::env::current_dir().unwrap_or_default().join("logs");
    let _ = std::fs::create_dir_all(&folder);
    // 固定文件层 logs/hw.log（无日期后缀，阻塞写入，无需 guard）
    // ReadWrite：允许 write_result_line 把 R<...>R 结果追加为该文件最后一行
    let roll = e_log::appender::rolling::never(&folder, "hw.log", e_log::FileShare::ReadWrite);
    let file_layer = e_log::subscriber::fmt::layer().with_writer(roll).with_ansi(false).with_target(false);
    // 控制台层（stderr，避免污染 stdout 协议流）
    let console_layer = e_log::subscriber::fmt::layer().with_writer(std::io::stderr).with_ansi(false).with_target(false);
    let sub = e_log::subscriber::registry().with(file_layer).with(console_layer);
    e_log::init_subscriber(sub, false);
    e_log::info!("日志初始化完成: {}", folder.display());
  }
}
