use crate::core::events::{EventBus, InternalEvent};
use crate::core::state::TimerState;
use crate::core::suppression::{SuppressionEngine, SuppressionReason, SuppressionStatus};
use crate::error::{AppError, AppResult};
use crate::services::config::{AppSettings, SuppressionMode};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchedulerSnapshot {
    pub state: TimerState,
    pub work_elapsed_seconds: u64,
    pub work_remaining_seconds: u64,
    pub rest_remaining_seconds: u64,
    pub settings: AppSettings,
}

#[derive(Debug)]
struct SchedulerInner {
    state: TimerState,
    settings: AppSettings,
    work_elapsed_seconds: u64,
    rest_remaining_seconds: u64,
}

#[derive(Debug, Clone)]
pub struct ReminderScheduler {
    inner: Arc<Mutex<SchedulerInner>>,
    event_bus: EventBus,
    suppression: SuppressionEngine,
}

impl ReminderScheduler {
    pub fn new(settings: AppSettings, event_bus: EventBus, suppression: SuppressionEngine) -> Self {
        Self {
            inner: Arc::new(Mutex::new(SchedulerInner {
                state: TimerState::Initializing,
                settings,
                work_elapsed_seconds: 0,
                rest_remaining_seconds: 0,
            })),
            event_bus,
            suppression,
        }
    }

    pub async fn start(&self) {
        {
            let mut inner = self.inner.lock().await;
            inner.state = if inner.settings.pause_reminders {
                TimerState::PausedByUser
            } else {
                TimerState::Working
            };
            self.event_bus
                .publish(InternalEvent::StateChanged { state: inner.state });
            self.event_bus.publish(InternalEvent::WorkIntervalStarted {
                duration_seconds: inner.settings.work_interval_seconds,
            });
        }

        let scheduler = self.clone();
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(1));
            loop {
                ticker.tick().await;
                scheduler.tick().await;
            }
        });
    }

    pub async fn snapshot(&self) -> SchedulerSnapshot {
        let inner = self.inner.lock().await;
        let work_remaining_seconds = inner
            .settings
            .work_interval_seconds
            .saturating_sub(inner.work_elapsed_seconds);

        SchedulerSnapshot {
            state: inner.state,
            work_elapsed_seconds: inner.work_elapsed_seconds,
            work_remaining_seconds,
            rest_remaining_seconds: inner.rest_remaining_seconds,
            settings: inner.settings.clone(),
        }
    }

    pub async fn update_settings(&self, settings: AppSettings) -> AppResult<()> {
        self.suppression
            .set_reason(SuppressionReason::PausedByUser, settings.pause_reminders)
            .await;

        {
            let mut inner = self.inner.lock().await;
            let was_paused = inner.state == TimerState::PausedByUser;
            inner.settings = settings.clone();

            if settings.pause_reminders && !was_paused {
                inner.state = TimerState::PausedByUser;
                self.event_bus
                    .publish(InternalEvent::StateChanged { state: inner.state });
            } else if !settings.pause_reminders && was_paused {
                inner.state = TimerState::Working;
                self.event_bus
                    .publish(InternalEvent::StateChanged { state: inner.state });
            }
        }

        self.event_bus
            .publish(InternalEvent::SettingsChanged(settings));
        Ok(())
    }

    pub async fn handle_suppression_status(&self, status: SuppressionStatus) {
        let mut inner = self.inner.lock().await;
        match inner.state {
            TimerState::Working if has_idle_suspension(&status) => {
                inner.state = TimerState::IdleSuspended;
                self.event_bus.publish(InternalEvent::IdleEntered);
                self.event_bus
                    .publish(InternalEvent::StateChanged { state: inner.state });
            }
            TimerState::IdleSuspended if !has_idle_suspension(&status) => {
                inner.state = TimerState::Working;
                self.event_bus.publish(InternalEvent::IdleExited);
                self.event_bus
                    .publish(InternalEvent::StateChanged { state: inner.state });
            }
            TimerState::ReminderPending if !status.suppressed => {
                inner.state = TimerState::ReminderShown;
                self.event_bus
                    .publish(InternalEvent::StateChanged { state: inner.state });
                self.event_bus.publish(InternalEvent::ReminderShown);
            }
            TimerState::ReminderShown if has_disruptive_suppression(&status) => {
                inner.state = TimerState::ReminderPending;
                self.event_bus
                    .publish(InternalEvent::StateChanged { state: inner.state });
                self.event_bus
                    .publish(InternalEvent::ReminderPending(status));
            }
            _ => {}
        }
    }

    pub async fn skip_reminder(&self) {
        let mut inner = self.inner.lock().await;
        if matches!(
            inner.state,
            TimerState::ReminderShown | TimerState::ReminderPending
        ) {
            self.event_bus.publish(InternalEvent::ReminderSkipped);
            Self::start_new_work_interval(&mut inner, &self.event_bus);
        }
    }

    pub async fn start_rest(&self) -> AppResult<()> {
        let mut inner = self.inner.lock().await;
        if !matches!(inner.state, TimerState::ReminderShown | TimerState::Resting) {
            return Err(AppError::from(
                "rest can only start while a reminder is shown",
            ));
        }
        Self::start_rest_locked(&mut inner, &self.event_bus);
        Ok(())
    }

    pub async fn start_rest_now(&self) {
        let mut inner = self.inner.lock().await;
        inner.state = TimerState::ReminderShown;
        self.event_bus.publish(InternalEvent::ReminderShown);
        Self::start_rest_locked(&mut inner, &self.event_bus);
    }

    pub async fn cancel_rest(&self) {
        let mut inner = self.inner.lock().await;
        if inner.state == TimerState::Resting {
            self.event_bus.publish(InternalEvent::RestCanceled);
            Self::start_new_work_interval(&mut inner, &self.event_bus);
        }
    }

    async fn tick(&self) {
        let mut inner = self.inner.lock().await;
        match inner.state {
            TimerState::Working => self.tick_working(&mut inner).await,
            TimerState::ReminderPending => self.tick_pending(&mut inner).await,
            TimerState::Resting => Self::tick_resting(&mut inner, &self.event_bus),
            TimerState::PausedByUser
            | TimerState::Initializing
            | TimerState::IdleSuspended
            | TimerState::ReminderShown
            | TimerState::Exiting => {}
        }
    }

    async fn tick_working(&self, inner: &mut SchedulerInner) {
        if inner.settings.pause_reminders {
            inner.state = TimerState::PausedByUser;
            self.event_bus
                .publish(InternalEvent::StateChanged { state: inner.state });
            return;
        }

        inner.work_elapsed_seconds = inner.work_elapsed_seconds.saturating_add(1);
        if inner.work_elapsed_seconds < inner.settings.work_interval_seconds {
            return;
        }

        self.event_bus.publish(InternalEvent::WorkIntervalElapsed);
        self.handle_interval_elapsed(inner).await;
    }

    async fn tick_pending(&self, inner: &mut SchedulerInner) {
        if inner.settings.suppression_mode == SuppressionMode::Skip {
            Self::start_new_work_interval(inner, &self.event_bus);
            return;
        }

        let status = self.suppression.status().await;
        self.event_bus
            .publish(InternalEvent::SuppressionChanged(status.clone()));
        if !status.suppressed {
            inner.state = TimerState::ReminderShown;
            self.event_bus
                .publish(InternalEvent::StateChanged { state: inner.state });
            self.event_bus.publish(InternalEvent::ReminderShown);
        }
    }

    async fn handle_interval_elapsed(&self, inner: &mut SchedulerInner) {
        self.event_bus.publish(InternalEvent::ReminderDue);
        let status = self.suppression.status().await;
        self.event_bus
            .publish(InternalEvent::SuppressionChanged(status.clone()));

        if !status.suppressed {
            inner.state = TimerState::ReminderShown;
            self.event_bus
                .publish(InternalEvent::StateChanged { state: inner.state });
            self.event_bus.publish(InternalEvent::ReminderShown);
            return;
        }

        match inner.settings.suppression_mode {
            SuppressionMode::Delay => {
                inner.state = TimerState::ReminderPending;
                self.event_bus
                    .publish(InternalEvent::StateChanged { state: inner.state });
                self.event_bus
                    .publish(InternalEvent::ReminderPending(status));
            }
            SuppressionMode::Skip => {
                Self::start_new_work_interval(inner, &self.event_bus);
            }
        }
    }

    fn tick_resting(inner: &mut SchedulerInner, event_bus: &EventBus) {
        if inner.rest_remaining_seconds > 0 {
            inner.rest_remaining_seconds -= 1;
            event_bus.publish(InternalEvent::RestTick {
                remaining_seconds: inner.rest_remaining_seconds,
            });
        }

        if inner.rest_remaining_seconds == 0 {
            event_bus.publish(InternalEvent::RestCompleted);
            Self::start_new_work_interval(inner, event_bus);
        }
    }

    fn start_rest_locked(inner: &mut SchedulerInner, event_bus: &EventBus) {
        inner.state = TimerState::Resting;
        inner.rest_remaining_seconds = inner.settings.rest_duration_seconds;
        event_bus.publish(InternalEvent::StateChanged { state: inner.state });
        event_bus.publish(InternalEvent::RestStarted {
            duration_seconds: inner.settings.rest_duration_seconds,
        });
        event_bus.publish(InternalEvent::RestTick {
            remaining_seconds: inner.rest_remaining_seconds,
        });
    }

    fn start_new_work_interval(inner: &mut SchedulerInner, event_bus: &EventBus) {
        inner.state = TimerState::Working;
        inner.work_elapsed_seconds = 0;
        inner.rest_remaining_seconds = 0;
        event_bus.publish(InternalEvent::StateChanged { state: inner.state });
        event_bus.publish(InternalEvent::WorkIntervalStarted {
            duration_seconds: inner.settings.work_interval_seconds,
        });
    }
}

fn has_idle_suspension(status: &SuppressionStatus) -> bool {
    status.reasons.iter().any(|reason| {
        matches!(
            reason,
            SuppressionReason::Idle | SuppressionReason::SessionLocked
        )
    })
}

fn has_disruptive_suppression(status: &SuppressionStatus) -> bool {
    status.reasons.iter().any(|reason| {
        matches!(
            reason,
            SuppressionReason::FullscreenApp | SuppressionReason::WhitelistedProcess
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scheduler() -> ReminderScheduler {
        let event_bus = EventBus::new();
        let suppression = SuppressionEngine::new();
        ReminderScheduler::new(AppSettings::default(), event_bus, suppression)
    }

    #[tokio::test]
    async fn cancel_rest_returns_to_working() {
        let scheduler = scheduler();

        scheduler.start_rest_now().await;
        scheduler.cancel_rest().await;

        let snapshot = scheduler.snapshot().await;
        assert_eq!(snapshot.state, TimerState::Working);
        assert_eq!(snapshot.rest_remaining_seconds, 0);
    }

    #[tokio::test]
    async fn idle_status_suspends_and_resumes_working_timer() {
        let scheduler = scheduler();
        {
            let mut inner = scheduler.inner.lock().await;
            inner.state = TimerState::Working;
            inner.work_elapsed_seconds = 12;
        }

        scheduler
            .handle_suppression_status(SuppressionStatus {
                suppressed: true,
                reasons: vec![SuppressionReason::Idle],
            })
            .await;

        let snapshot = scheduler.snapshot().await;
        assert_eq!(snapshot.state, TimerState::IdleSuspended);
        assert_eq!(snapshot.work_elapsed_seconds, 12);

        scheduler
            .handle_suppression_status(SuppressionStatus {
                suppressed: false,
                reasons: Vec::new(),
            })
            .await;

        assert_eq!(scheduler.snapshot().await.state, TimerState::Working);
    }

    #[tokio::test]
    async fn fullscreen_suppression_hides_shown_reminder_to_pending() {
        let scheduler = scheduler();
        {
            let mut inner = scheduler.inner.lock().await;
            inner.state = TimerState::ReminderShown;
        }

        scheduler
            .handle_suppression_status(SuppressionStatus {
                suppressed: true,
                reasons: vec![SuppressionReason::FullscreenApp],
            })
            .await;

        assert_eq!(
            scheduler.snapshot().await.state,
            TimerState::ReminderPending
        );
    }

    #[tokio::test]
    async fn cleared_fullscreen_suppression_shows_pending_reminder() {
        let scheduler = scheduler();
        {
            let mut inner = scheduler.inner.lock().await;
            inner.state = TimerState::ReminderPending;
        }

        scheduler
            .handle_suppression_status(SuppressionStatus {
                suppressed: false,
                reasons: Vec::new(),
            })
            .await;

        assert_eq!(scheduler.snapshot().await.state, TimerState::ReminderShown);
    }
}
