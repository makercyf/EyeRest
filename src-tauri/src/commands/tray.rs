use crate::app::EyeRestState;
use crate::error::AppResult;
use tauri::State;

#[tauri::command]
pub async fn pause_reminders(state: State<'_, EyeRestState>) -> AppResult<()> {
    let mut settings = state.config.get().await;
    settings.pause_reminders = true;
    let saved = state.config.save(settings).await?;
    state.scheduler.update_settings(saved).await?;
    Ok(())
}

#[tauri::command]
pub async fn resume_reminders(state: State<'_, EyeRestState>) -> AppResult<()> {
    let mut settings = state.config.get().await;
    settings.pause_reminders = false;
    let saved = state.config.save(settings).await?;
    state.scheduler.update_settings(saved).await?;
    Ok(())
}
