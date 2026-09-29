use std::path::Path;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Release builds run all day, every day: keep the file to meaningful events.
const DEFAULT_FILTER: &str = if cfg!(debug_assertions) {
    "info,notch_desktop_lib=debug"
} else {
    "warn,notch_desktop_lib=info"
};
/// Daily files kept on disk; older ones are deleted automatically.
const MAX_LOG_FILES: usize = 7;

/// Keeps the non-blocking log writer alive for the lifetime of the app.
pub struct LogGuard(#[allow(dead_code)] WorkerGuard);

/// Structured, local-only logging: a daily-rolling file in the app log dir,
/// plus stdout in debug builds. Callers must log identifiers and event types,
/// never message bodies, commands, or credentials (architecture.md §11–12).
pub fn init(log_dir: &Path) -> LogGuard {
    let file_appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("notch.log")
        .max_log_files(MAX_LOG_FILES)
        .build(log_dir)
        .unwrap_or_else(|_| tracing_appender::rolling::daily(log_dir, "notch.log"));
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_FILTER));

    let file_layer = fmt::layer()
        .with_writer(file_writer)
        .with_ansi(false)
        .with_target(true);
    let stdout_layer = cfg!(debug_assertions).then(|| fmt::layer().with_target(true));

    // `try_init` so tests or a second init call don't panic.
    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(file_layer)
        .with(stdout_layer)
        .try_init();

    LogGuard(guard)
}
