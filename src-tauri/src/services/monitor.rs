use crate::error::{AppError, AppResult};
use crate::services::config::AppSettings;
use serde::Serialize;
use std::collections::HashMap;
use tauri::AppHandle;
#[cfg(target_os = "windows")]
use windows::Win32::Devices::Display::{
    DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes, QueryDisplayConfig,
    DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
    DISPLAYCONFIG_DEVICE_INFO_HEADER, DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO,
    DISPLAYCONFIG_SOURCE_DEVICE_NAME, DISPLAYCONFIG_TARGET_DEVICE_NAME, QDC_ONLY_ACTIVE_PATHS,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    pub id: String,
    pub name: String,
    pub is_primary: bool,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
}

#[derive(Debug, Clone)]
pub struct OverlayMonitor {
    pub id: String,
    pub label: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

pub fn list_monitors(app: &AppHandle) -> AppResult<Vec<MonitorInfo>> {
    let monitors = app.available_monitors()?;
    if monitors.is_empty() {
        return Err(AppError::from("monitor enumeration returned no monitors"));
    }

    let primary_id = app.primary_monitor()?.map(|monitor| monitor_id(&monitor));
    let monitor_models = monitor_model_names();

    Ok(monitors
        .into_iter()
        .enumerate()
        .map(|(index, monitor)| {
            let id = monitor_id(&monitor);
            let size = monitor.size();
            let position = monitor.position();
            let fallback_name = monitor
                .name()
                .cloned()
                .unwrap_or_else(|| format!("Monitor {}", index + 1));
            MonitorInfo {
                id: id.clone(),
                name: monitor_models
                    .get(&fallback_name.to_ascii_uppercase())
                    .cloned()
                    .unwrap_or(fallback_name),
                is_primary: primary_id.as_ref() == Some(&id),
                x: position.x,
                y: position.y,
                width: size.width,
                height: size.height,
                scale_factor: monitor.scale_factor(),
            }
        })
        .collect())
}

#[cfg(target_os = "windows")]
fn monitor_model_names() -> HashMap<String, String> {
    let mut path_count = 0;
    let mut mode_count = 0;
    if unsafe {
        GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut path_count, &mut mode_count)
    }
    .0 != 0
    {
        return HashMap::new();
    }

    let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
    let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];
    if unsafe {
        QueryDisplayConfig(
            QDC_ONLY_ACTIVE_PATHS,
            &mut path_count,
            paths.as_mut_ptr(),
            &mut mode_count,
            modes.as_mut_ptr(),
            None,
        )
    }
    .0 != 0
    {
        return HashMap::new();
    }

    paths.truncate(path_count as usize);
    paths
        .into_iter()
        .filter_map(|path| {
            let mut source = DISPLAYCONFIG_SOURCE_DEVICE_NAME {
                header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                    r#type: DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
                    size: std::mem::size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32,
                    adapterId: path.sourceInfo.adapterId,
                    id: path.sourceInfo.id,
                },
                ..Default::default()
            };
            if unsafe { DisplayConfigGetDeviceInfo(&mut source.header) } != 0 {
                return None;
            }

            let mut target = DISPLAYCONFIG_TARGET_DEVICE_NAME {
                header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                    r#type: DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
                    size: std::mem::size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32,
                    adapterId: path.targetInfo.adapterId,
                    id: path.targetInfo.id,
                },
                ..Default::default()
            };
            if unsafe { DisplayConfigGetDeviceInfo(&mut target.header) } != 0 {
                return None;
            }

            let display_name = wide_string(&source.viewGdiDeviceName);
            let model_name = wide_string(&target.monitorFriendlyDeviceName);
            is_friendly_monitor_name(&model_name)
                .then_some((display_name.to_ascii_uppercase(), model_name))
        })
        .collect()
}

#[cfg(not(target_os = "windows"))]
fn monitor_model_names() -> HashMap<String, String> {
    HashMap::new()
}

#[cfg(target_os = "windows")]
fn wide_string(value: &[u16]) -> String {
    let length = value
        .iter()
        .position(|&character| character == 0)
        .unwrap_or(value.len());
    String::from_utf16_lossy(&value[..length]).trim().to_owned()
}

#[cfg(target_os = "windows")]
fn is_friendly_monitor_name(name: &str) -> bool {
    !name.is_empty()
        && !name.eq_ignore_ascii_case("Generic PnP Monitor")
        && !name.eq_ignore_ascii_case("Unknown")
}

pub fn selected_overlay_monitors(
    app: &AppHandle,
    settings: &AppSettings,
) -> AppResult<Vec<OverlayMonitor>> {
    let monitors = app.available_monitors()?;
    if monitors.is_empty() {
        return Err(AppError::from("monitor enumeration returned no monitors"));
    }

    let selected = monitors
        .into_iter()
        .filter_map(|monitor| {
            let id = monitor_id(&monitor);
            if settings.monitor_mode == "selected" && !settings.selected_monitor_ids.contains(&id) {
                return None;
            }

            let size = monitor.size();
            let position = monitor.position();
            Some(OverlayMonitor {
                label: overlay_label(&id),
                id,
                x: position.x,
                y: position.y,
                width: size.width,
                height: size.height,
            })
        })
        .collect::<Vec<_>>();

    if selected.is_empty() {
        let primary = app
            .primary_monitor()?
            .or_else(|| {
                app.available_monitors()
                    .ok()
                    .and_then(|mut monitors| monitors.pop())
            })
            .ok_or_else(|| AppError::from("no monitor available for overlay"))?;
        let id = monitor_id(&primary);
        let size = primary.size();
        let position = primary.position();
        return Ok(vec![OverlayMonitor {
            label: overlay_label(&id),
            id,
            x: position.x,
            y: position.y,
            width: size.width,
            height: size.height,
        }]);
    }

    Ok(selected)
}

pub fn overlay_label(monitor_id: &str) -> String {
    let safe_id = monitor_id
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    format!("overlay-{safe_id}")
}

pub fn is_overlay_label(label: &str) -> bool {
    label.starts_with("overlay-")
}

fn monitor_id(monitor: &tauri::Monitor) -> String {
    let size = monitor.size();
    let position = monitor.position();
    let name = monitor
        .name()
        .map(|name| sanitize_id_part(name))
        .unwrap_or_else(|| "unnamed".into());

    format!(
        "{name}-{}x{}@{},{}",
        size.width, size.height, position.x, position.y
    )
}

fn sanitize_id_part(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}
