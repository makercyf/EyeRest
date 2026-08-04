use crate::commands;
use crate::core::events::{EventBus, InternalEvent};
use crate::core::scheduler::ReminderScheduler;
use crate::core::suppression::SuppressionEngine;
use crate::core::suppression::{SuppressionReason, SuppressionStatus};
use crate::detectors::{fullscreen, idle, whitelist};
use crate::platform;
use crate::services::{
    audio, autostart,
    config::ConfigService,
    logging::{self, DiagnosticSeverity},
    overlay, tray,
};
use tauri::{Emitter, Manager};
use tokio::time::{interval, Duration};

#[derive(Clone)]
pub struct EyeRestState {
    pub config: ConfigService,
    pub scheduler: ReminderScheduler,
}

pub fn run() {
    if let Err(error) = logging::init() {
        eprintln!("EyeRest logging unavailable: {error}");
    }

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::pick_whitelisted_executable,
            commands::get_app_status,
            commands::open_settings_window,
            commands::dismiss_overlay,
            commands::list_monitors,
            commands::start_rest,
            commands::skip_reminder,
            commands::cancel_rest,
            commands::start_rest_now,
            commands::pause_reminders,
            commands::resume_reminders
        ])
        .setup(|app| {
            let (config, load_report) = ConfigService::load()
                .map_err(|error| tauri::Error::Anyhow(anyhow::anyhow!(error.to_string())))?;
            let settings = tauri::async_runtime::block_on(config.get());
            if let Err(error) = autostart::sync(settings.autostart) {
                logging::record_diagnostic(
                    DiagnosticSeverity::Warning,
                    format!("Windows startup setting could not be applied: {error}"),
                );
            }
            let event_bus = EventBus::new();
            let suppression = SuppressionEngine::new();
            let scheduler =
                ReminderScheduler::new(settings.clone(), event_bus.clone(), suppression.clone());

            let state = EyeRestState {
                config: config.clone(),
                scheduler: scheduler.clone(),
            };

            app.manage(state);
            if let Some(window) = app.get_webview_window(overlay::SETTINGS_LABEL) {
                platform::windows::apply_dpi_aware_webview_icons(&window)
                    .map_err(|error| tauri::Error::Anyhow(anyhow::anyhow!(error.to_string())))?;
            }
            tray::create_tray(app.handle())
                .map_err(|error| tauri::Error::Anyhow(anyhow::anyhow!(error.to_string())))?;

            spawn_event_router(app.handle().clone(), event_bus.clone(), config.clone());
            spawn_detector_loop(
                config.clone(),
                scheduler.clone(),
                event_bus.clone(),
                suppression.clone(),
            );

            event_bus.publish(InternalEvent::SettingsLoaded);
            event_bus.publish(InternalEvent::SettingsChanged(settings));
            if load_report.recovered_from_corrupt_file {
                logging::record_diagnostic(
                    DiagnosticSeverity::Warning,
                    "Settings were corrupt and have been reset to defaults.",
                );
                tracing::warn!("settings recovered at {:?}", load_report.settings_path);
            }
            if load_report.newer_version_ignored {
                logging::record_diagnostic(
                    DiagnosticSeverity::Warning,
                    "Settings came from a newer app version; safe defaults are being used.",
                );
                tracing::warn!(
                    "newer settings version ignored at {:?}",
                    load_report.settings_path
                );
            }

            tauri::async_runtime::spawn(async move {
                scheduler.start().await;
                event_bus.publish(InternalEvent::AppStarted);
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == overlay::SETTINGS_LABEL {
                if matches!(event, tauri::WindowEvent::ScaleFactorChanged { .. }) {
                    if let Err(error) = platform::windows::apply_dpi_aware_window_icons(window) {
                        logging::record_diagnostic(
                            DiagnosticSeverity::Warning,
                            format!("Taskbar icon could not be updated for the new DPI: {error}"),
                        );
                    }
                }
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running EyeRest");
}

fn spawn_detector_loop(
    config: ConfigService,
    scheduler: ReminderScheduler,
    event_bus: EventBus,
    suppression: SuppressionEngine,
) {
    tauri::async_runtime::spawn(async move {
        let mut ticker = interval(Duration::from_secs(1));
        let mut previous_status = SuppressionStatus {
            suppressed: false,
            reasons: Vec::new(),
        };
        let mut previous_idle = false;
        let mut previous_fullscreen = false;
        let mut previous_whitelist = false;

        loop {
            ticker.tick().await;
            let settings = config.get().await;

            let idle_active = settings.idle_suppression_enabled
                && idle::is_idle(settings.idle_threshold_seconds).unwrap_or_else(|error| {
                    logging::record_diagnostic(
                        DiagnosticSeverity::Warning,
                        format!("Idle detector unavailable this check: {error}"),
                    );
                    tracing::warn!("idle detector unavailable: {error}");
                    false
                });
            let fullscreen_active = settings.fullscreen_suppression_enabled
                && fullscreen::is_any_fullscreen_window().unwrap_or_else(|error| {
                    logging::record_diagnostic(
                        DiagnosticSeverity::Warning,
                        format!("Fullscreen detector error: {error}"),
                    );
                    tracing::warn!("fullscreen detector error: {error}");
                    false
                });
            let foreground_process = whitelist::foreground_process_name().unwrap_or_else(|error| {
                logging::record_diagnostic(
                    DiagnosticSeverity::Warning,
                    format!("Foreground process lookup failed: {error}"),
                );
                tracing::warn!("foreground process lookup failed: {error}");
                None
            });
            let whitelist_active = foreground_process.is_some_and(|process_name| {
                settings
                    .whitelisted_processes
                    .iter()
                    .any(|entry| entry.eq_ignore_ascii_case(&process_name))
            });

            if idle_active != previous_idle {
                event_bus.publish(if idle_active {
                    InternalEvent::IdleEntered
                } else {
                    InternalEvent::IdleExited
                });
                previous_idle = idle_active;
            }

            if fullscreen_active != previous_fullscreen {
                event_bus.publish(if fullscreen_active {
                    InternalEvent::FullscreenEntered
                } else {
                    InternalEvent::FullscreenExited
                });
                previous_fullscreen = fullscreen_active;
            }

            if whitelist_active != previous_whitelist {
                event_bus.publish(if whitelist_active {
                    InternalEvent::WhitelistSuppressionEntered
                } else {
                    InternalEvent::WhitelistSuppressionExited
                });
                previous_whitelist = whitelist_active;
            }

            suppression.set_reason(idle::reason(), idle_active).await;
            suppression
                .set_reason(fullscreen::reason(), fullscreen_active)
                .await;
            suppression
                .set_reason(whitelist::reason(), whitelist_active)
                .await;
            suppression
                .set_reason(SuppressionReason::PausedByUser, settings.pause_reminders)
                .await;

            let status = suppression.status().await;
            if status != previous_status {
                previous_status = status.clone();
                event_bus.publish(InternalEvent::SuppressionChanged(status.clone()));
                scheduler.handle_suppression_status(status).await;
            }
        }
    });
}

fn spawn_event_router(app: tauri::AppHandle, event_bus: EventBus, config: ConfigService) {
    let mut receiver = event_bus.subscribe();
    tauri::async_runtime::spawn(async move {
        while let Ok(event) = receiver.recv().await {
            if let Err(error) = app.emit("eyerest://event", event.clone()) {
                tracing::warn!("failed to emit frontend event: {error}");
            }

            match event {
                InternalEvent::ReminderShown => {
                    let settings = config.get().await;
                    if let Err(error) = overlay::show_overlay_windows(&app, &settings) {
                        event_bus.publish(InternalEvent::OverlayFailed {
                            message: error.to_string(),
                        });
                        logging::record_diagnostic(
                            DiagnosticSeverity::Error,
                            format!("Overlay window creation failed: {error}"),
                        );
                    }
                }
                InternalEvent::ReminderSkipped
                | InternalEvent::RestCanceled
                | InternalEvent::RestCompleted
                | InternalEvent::ReminderPending(_)
                | InternalEvent::AppExiting => {
                    if let Err(error) = overlay::close_overlay_windows(&app) {
                        event_bus.publish(InternalEvent::OverlayFailed {
                            message: error.to_string(),
                        });
                        logging::record_diagnostic(
                            DiagnosticSeverity::Error,
                            format!("Overlay window close failed: {error}"),
                        );
                    }

                    if matches!(event, InternalEvent::RestCompleted) {
                        let settings = config.get().await;
                        if let Err(error) = audio::play_rest_complete_sound(settings.sound_enabled)
                        {
                            event_bus.publish(InternalEvent::AudioFailed {
                                message: error.to_string(),
                            });
                            logging::record_diagnostic(
                                DiagnosticSeverity::Warning,
                                format!("Audio playback failed: {error}"),
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    });
}
