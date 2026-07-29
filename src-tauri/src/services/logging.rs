use crate::error::{AppError, AppResult};
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Duration, SystemTime};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::Rotation;
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;

const LOG_RETENTION_DAYS: u64 = 7;

static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();
#[derive(Debug, Clone)]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

pub fn init() -> AppResult<PathBuf> {
    let log_dir = log_directory()?;
    fs::create_dir_all(&log_dir)?;
    prune_old_logs(&log_dir)?;

    let file_appender = tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("EyeRest")
        .filename_suffix("log")
        .build(&log_dir)
        .map_err(|error| AppError::from(format!("log file initialization failed: {error}")))?;
    let (writer, guard) = tracing_appender::non_blocking(file_appender);
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let subscriber = fmt::layer()
        .with_writer(writer)
        .with_ansi(false)
        .with_target(false);
    tracing_subscriber::registry()
        .with(env_filter)
        .with(subscriber)
        .try_init()
        .map_err(|error| AppError::from(format!("logging initialization failed: {error}")))?;
    let _ = LOG_GUARD.set(guard);

    record_diagnostic(
        DiagnosticSeverity::Info,
        format!("Debug logging initialized at {}", log_dir.display()),
    );
    Ok(log_dir)
}

pub fn record_diagnostic(severity: DiagnosticSeverity, message: impl Into<String>) {
    let message = message.into();
    match severity {
        DiagnosticSeverity::Info => tracing::info!("{message}"),
        DiagnosticSeverity::Warning => tracing::warn!("{message}"),
        DiagnosticSeverity::Error => tracing::error!("{message}"),
    }
}

pub fn log_directory() -> AppResult<PathBuf> {
    if portable_marker_exists() {
        let exe_dir = std::env::current_exe()?
            .parent()
            .map(|path| path.to_path_buf())
            .ok_or_else(|| AppError::from("executable directory is unavailable"))?;
        return Ok(exe_dir.join("logs"));
    }

    if let Some(base) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) {
        return Ok(base.join("EyeRest").join("logs"));
    }

    Ok(std::env::current_dir()?.join("logs"))
}

fn portable_marker_exists() -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|path| path.join("EyeRest.portable")))
        .is_some_and(|marker| marker.exists())
}

fn prune_old_logs(log_dir: &PathBuf) -> AppResult<()> {
    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(LOG_RETENTION_DAYS * 24 * 60 * 60))
        .unwrap_or(SystemTime::UNIX_EPOCH);

    if !log_dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(log_dir)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if !metadata.is_file() {
            continue;
        }

        let path = entry.path();
        let is_log = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                (name.starts_with("EyeRest.") && name.ends_with(".log"))
                    || name.starts_with("EyeRest.log.")
            });
        if is_log && metadata.modified().is_ok_and(|modified| modified < cutoff) {
            let _ = fs::remove_file(path);
        }
    }

    Ok(())
}
