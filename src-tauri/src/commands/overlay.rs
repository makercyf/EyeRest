use crate::app::EyeRestState;
use crate::core::scheduler::SchedulerSnapshot;
use crate::error::AppResult;
use crate::services::{monitor, overlay};
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn get_app_status(state: State<'_, EyeRestState>) -> AppResult<SchedulerSnapshot> {
    Ok(state.scheduler.snapshot().await)
}

#[tauri::command]
pub async fn open_settings_window(app: AppHandle) -> AppResult<()> {
    overlay::show_settings_window(&app)
}

#[tauri::command]
pub async fn dismiss_overlay(app: AppHandle) -> AppResult<()> {
    overlay::close_overlay_windows(&app)
}

#[tauri::command]
pub async fn list_monitors(app: AppHandle) -> AppResult<Vec<monitor::MonitorInfo>> {
    monitor::list_monitors(&app)
}

#[tauri::command]
pub async fn start_rest(state: State<'_, EyeRestState>) -> AppResult<()> {
    state.scheduler.start_rest().await
}

#[tauri::command]
pub async fn skip_reminder(state: State<'_, EyeRestState>) -> AppResult<()> {
    state.scheduler.skip_reminder().await;
    Ok(())
}

#[tauri::command]
pub async fn cancel_rest(state: State<'_, EyeRestState>) -> AppResult<()> {
    state.scheduler.cancel_rest().await;
    Ok(())
}

#[tauri::command]
pub async fn start_rest_now(state: State<'_, EyeRestState>) -> AppResult<()> {
    state.scheduler.start_rest_now().await;
    Ok(())
}
