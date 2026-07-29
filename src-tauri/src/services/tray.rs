use crate::app::EyeRestState;
use crate::error::AppResult;
use crate::services::overlay;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

const OPEN_SETTINGS_ID: &str = "open_settings";
const PAUSE_ID: &str = "pause_reminders";
const RESUME_ID: &str = "resume_reminders";
const START_REST_ID: &str = "start_rest_now";
const QUIT_ID: &str = "quit";

pub fn create_tray(app: &AppHandle) -> AppResult<()> {
    let open_settings =
        MenuItem::with_id(app, OPEN_SETTINGS_ID, "Open Settings", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, PAUSE_ID, "Pause Reminders", true, None::<&str>)?;
    let resume = MenuItem::with_id(app, RESUME_ID, "Resume Reminders", true, None::<&str>)?;
    let start_rest = MenuItem::with_id(app, START_REST_ID, "Start Rest Now", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, QUIT_ID, "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&start_rest, &resume, &pause, &open_settings, &quit])?;

    let icon = app
        .default_window_icon()
        .map(|icon| icon.clone().to_owned())
        .unwrap_or_else(tray_icon);
    TrayIconBuilder::with_id("eyerest-tray")
        .tooltip("EyeRest")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::DoubleClick {
                button: MouseButton::Left,
                ..
            } = event
            {
                let _ = overlay::show_settings_window(tray.app_handle());
            }
        })
        .on_menu_event(|app, event| {
            let app = app.clone();
            match event.id().as_ref() {
                OPEN_SETTINGS_ID => {
                    let _ = overlay::show_settings_window(&app);
                }
                PAUSE_ID => {
                    tauri::async_runtime::spawn(async move {
                        if let Some(state) = app.try_state::<EyeRestState>() {
                            let mut settings = state.config.get().await;
                            settings.pause_reminders = true;
                            if let Ok(saved) = state.config.save(settings).await {
                                let _ = state.scheduler.update_settings(saved).await;
                            }
                        }
                    });
                }
                RESUME_ID => {
                    tauri::async_runtime::spawn(async move {
                        if let Some(state) = app.try_state::<EyeRestState>() {
                            let mut settings = state.config.get().await;
                            settings.pause_reminders = false;
                            if let Ok(saved) = state.config.save(settings).await {
                                let _ = state.scheduler.update_settings(saved).await;
                            }
                        }
                    });
                }
                START_REST_ID => {
                    tauri::async_runtime::spawn(async move {
                        if let Some(state) = app.try_state::<EyeRestState>() {
                            state.scheduler.start_rest_now().await;
                        }
                    });
                }
                QUIT_ID => app.exit(0),
                _ => {}
            }
        })
        .build(app)?;

    Ok(())
}

fn tray_icon() -> Image<'static> {
    let mut rgba = Vec::with_capacity(32 * 32 * 4);
    for y in 0..32 {
        for x in 0..32 {
            let dx = x as i32 - 16;
            let dy = y as i32 - 16;
            let in_circle = dx * dx + dy * dy <= 14 * 14;
            if in_circle {
                rgba.extend_from_slice(&[0x62, 0xd2, 0x6f, 0xff]);
            } else {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }
    Image::new_owned(rgba, 32, 32)
}
