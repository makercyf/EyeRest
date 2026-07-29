use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimerState {
    Initializing,
    Working,
    PausedByUser,
    IdleSuspended,
    ReminderPending,
    ReminderShown,
    Resting,
    Exiting,
}
