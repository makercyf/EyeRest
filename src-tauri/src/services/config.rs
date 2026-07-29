use crate::error::{AppError, AppResult};
use crate::services::theme::{self, OverlayTheme};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

pub const SETTINGS_VERSION: u16 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SuppressionMode {
    Delay,
    Skip,
}

impl Default for SuppressionMode {
    fn default() -> Self {
        Self::Delay
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub work_interval_seconds: u64,
    pub rest_duration_seconds: u64,
    pub idle_threshold_seconds: u64,
    pub suppression_mode: SuppressionMode,
    pub pause_reminders: bool,
    pub fullscreen_suppression_enabled: bool,
    pub idle_suppression_enabled: bool,
    pub whitelisted_processes: Vec<String>,
    pub monitor_mode: String,
    pub selected_monitor_ids: Vec<String>,
    pub autostart: bool,
    pub sound_enabled: bool,
    pub theme: OverlayTheme,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            work_interval_seconds: 20 * 60,
            rest_duration_seconds: 20,
            idle_threshold_seconds: 60,
            suppression_mode: SuppressionMode::Delay,
            pause_reminders: false,
            fullscreen_suppression_enabled: true,
            idle_suppression_enabled: true,
            whitelisted_processes: Vec::new(),
            monitor_mode: "all".into(),
            selected_monitor_ids: Vec::new(),
            autostart: false,
            sound_enabled: true,
            theme: OverlayTheme::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsDocument {
    pub version: u16,
    pub settings: AppSettings,
}

impl Default for SettingsDocument {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            settings: AppSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigLoadReport {
    pub settings_path: PathBuf,
    pub recovered_from_corrupt_file: bool,
    pub newer_version_ignored: bool,
}

#[derive(Debug, Clone)]
pub struct ConfigService {
    path: PathBuf,
    settings: Arc<RwLock<AppSettings>>,
}

impl ConfigService {
    pub fn load() -> AppResult<(Self, ConfigLoadReport)> {
        let path = settings_path()?;
        let mut report = ConfigLoadReport {
            settings_path: path.clone(),
            recovered_from_corrupt_file: false,
            newer_version_ignored: false,
        };

        if !path.exists() {
            let document = SettingsDocument::default();
            write_document(&path, &document)?;
            return Ok((Self::new(path, document.settings), report));
        }

        let raw = fs::read_to_string(&path)?;
        let document = match serde_json::from_str::<SettingsDocument>(&raw) {
            Ok(document) => document,
            Err(error) => {
                let backup = corrupt_backup_path(&path);
                fs::rename(&path, backup)?;
                let document = SettingsDocument::default();
                write_document(&path, &document)?;
                report.recovered_from_corrupt_file = true;
                tracing::warn!("settings recovery triggered: {error}");
                document
            }
        };

        if document.version > SETTINGS_VERSION {
            report.newer_version_ignored = true;
            return Ok((Self::new(path, AppSettings::default()), report));
        }

        if document.version < SETTINGS_VERSION {
            let upgraded = SettingsDocument {
                version: SETTINGS_VERSION,
                settings: document.settings,
            };
            write_document(&path, &upgraded)?;
            return Ok((Self::new(path, upgraded.settings), report));
        }

        Ok((Self::new(path, document.settings), report))
    }

    pub async fn get(&self) -> AppSettings {
        self.settings.read().await.clone()
    }

    pub async fn save(&self, settings: AppSettings) -> AppResult<AppSettings> {
        let mut settings = settings;
        settings.theme = settings.theme.normalized()?;
        settings.whitelisted_processes = normalize_process_names(settings.whitelisted_processes);
        validate_settings(&settings)?;
        let document = SettingsDocument {
            version: SETTINGS_VERSION,
            settings: settings.clone(),
        };
        write_document(&self.path, &document)?;
        *self.settings.write().await = settings.clone();
        Ok(settings)
    }

    fn new(path: PathBuf, settings: AppSettings) -> Self {
        Self {
            path,
            settings: Arc::new(RwLock::new(settings)),
        }
    }
}

fn validate_settings(settings: &AppSettings) -> AppResult<()> {
    if settings.work_interval_seconds == 0 {
        return Err(AppError::from("work interval must be at least 1 second"));
    }

    if settings.rest_duration_seconds == 0 {
        return Err(AppError::from("rest duration must be at least 1 second"));
    }

    if !matches!(settings.monitor_mode.as_str(), "all" | "selected") {
        return Err(AppError::from("monitor mode must be 'all' or 'selected'"));
    }

    theme::validate_theme(&settings.theme)?;

    Ok(())
}

fn normalize_process_names(processes: Vec<String>) -> Vec<String> {
    let mut processes = processes
        .into_iter()
        .map(|process| process.trim().to_ascii_uppercase())
        .filter(|process| !process.is_empty())
        .map(|process| {
            if process.ends_with(".EXE") {
                process
            } else {
                format!("{process}.EXE")
            }
        })
        .collect::<Vec<_>>();
    processes.sort();
    processes.dedup();
    processes
}

fn settings_path() -> AppResult<PathBuf> {
    if portable_marker_exists() {
        return Ok(executable_directory()?.join("data").join("settings.json"));
    }

    if let Some(base) = std::env::var_os("APPDATA").map(PathBuf::from) {
        return Ok(base.join("EyeRest").join("settings.json"));
    }

    Ok(executable_directory()?.join("data").join("settings.json"))
}

fn write_document(path: &PathBuf, document: &SettingsDocument) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let raw = serde_json::to_string_pretty(document)?;
    fs::write(path, raw)?;
    Ok(())
}

fn corrupt_backup_path(path: &PathBuf) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("settings.json");
    path.with_file_name(format!("{file_name}.corrupt.{timestamp}.bak"))
}

fn portable_marker_exists() -> bool {
    executable_directory()
        .map(|directory| directory.join("EyeRest.portable").exists())
        .unwrap_or(false)
}

fn executable_directory() -> AppResult<PathBuf> {
    std::env::current_exe()?
        .parent()
        .map(|path| path.to_path_buf())
        .ok_or_else(|| AppError::from("executable directory is unavailable"))
}
