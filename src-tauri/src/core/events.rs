use crate::core::state::TimerState;
use crate::core::suppression::SuppressionStatus;
use crate::services::config::AppSettings;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum InternalEvent {
    AppStarted,
    AppExiting,
    SettingsLoaded,
    SettingsChanged(AppSettings),
    WorkIntervalStarted { duration_seconds: u64 },
    WorkIntervalElapsed,
    ReminderDue,
    ReminderPending(SuppressionStatus),
    ReminderShown,
    ReminderSkipped,
    RestStarted { duration_seconds: u64 },
    RestTick { remaining_seconds: u64 },
    RestCanceled,
    RestCompleted,
    IdleEntered,
    IdleExited,
    FullscreenEntered,
    FullscreenExited,
    WhitelistSuppressionEntered,
    WhitelistSuppressionExited,
    SuppressionChanged(SuppressionStatus),
    StateChanged { state: TimerState },
    OverlayFailed { message: String },
    AudioFailed { message: String },
}

#[derive(Debug, Clone)]
pub struct EventBus {
    sender: broadcast::Sender<InternalEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(128);
        Self { sender }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<InternalEvent> {
        self.sender.subscribe()
    }

    pub fn publish(&self, event: InternalEvent) {
        let _ = self.sender.send(event);
    }
}
