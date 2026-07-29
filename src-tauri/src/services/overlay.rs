use crate::error::AppResult;
use crate::services::{config::AppSettings, monitor};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub const SETTINGS_LABEL: &str = "settings";

pub fn show_settings_window(app: &AppHandle) -> AppResult<()> {
    if let Some(window) = app.get_webview_window(SETTINGS_LABEL) {
        window.show()?;
        window.set_focus()?;
        return Ok(());
    }

    WebviewWindowBuilder::new(app, SETTINGS_LABEL, WebviewUrl::App("index.html".into()))
        .title("EyeRest Settings")
        .inner_size(920.0, 680.0)
        .resizable(true)
        .center()
        .build()?;

    Ok(())
}

pub fn show_overlay_windows(app: &AppHandle, settings: &AppSettings) -> AppResult<()> {
    close_overlay_windows(app)?;

    let monitors = monitor::selected_overlay_monitors(app, settings)?;
    for (index, monitor) in monitors.into_iter().enumerate() {
        let window = WebviewWindowBuilder::new(
            app,
            monitor.label.clone(),
            WebviewUrl::App("index.html?view=overlay".into()),
        )
        .title(format!("EyeRest - {}", monitor.id))
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .position(monitor.x as f64, monitor.y as f64)
        .inner_size(monitor.width as f64, monitor.height as f64)
        .focused(index == 0)
        .build()?;

        window.set_position(tauri::PhysicalPosition::new(monitor.x, monitor.y))?;
        window.set_size(tauri::PhysicalSize::new(monitor.width, monitor.height))?;
        // Windows may retain a small invisible resize frame on undecorated windows.
        // Native fullscreen uses the monitor's exact physical rectangle instead.
        window.set_fullscreen(true)?;
        window.show()?;
        if index == 0 {
            let _ = window.set_focus();
        }
    }

    Ok(())
}

pub fn close_overlay_windows(app: &AppHandle) -> AppResult<()> {
    let windows = app.webview_windows();
    for (label, window) in windows {
        if monitor::is_overlay_label(&label) {
            window.close()?;
        }
    }
    Ok(())
}
