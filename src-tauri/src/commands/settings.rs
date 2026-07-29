use crate::app::EyeRestState;
use crate::error::AppResult;
use crate::services::config::AppSettings;
use crate::services::{autostart, file_picker};
use tauri::State;

#[tauri::command]
pub async fn get_settings(state: State<'_, EyeRestState>) -> AppResult<AppSettings> {
    Ok(state.config.get().await)
}

#[tauri::command]
pub async fn save_settings(
    state: State<'_, EyeRestState>,
    settings: AppSettings,
) -> AppResult<AppSettings> {
    let previous = state.config.get().await;
    let saved = state.config.save(settings).await?;
    if let Err(error) = autostart::sync(saved.autostart) {
        state.config.save(previous).await?;
        return Err(error);
    }
    state.scheduler.update_settings(saved.clone()).await?;
    Ok(saved)
}

#[tauri::command]
pub fn pick_whitelisted_executable() -> AppResult<Option<String>> {
    file_picker::pick_executable_name()
}
