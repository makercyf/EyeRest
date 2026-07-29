#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderDefinition {
    pub id: String,
    pub reminder_type: ReminderType,
    pub enabled: bool,
    pub interval_seconds: u64,
    pub duration_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReminderType {
    EyeRest,
}

impl ReminderDefinition {
    pub fn eye_rest_default(interval_seconds: u64, duration_seconds: u64) -> Self {
        Self {
            id: "eye-rest-default".into(),
            reminder_type: ReminderType::EyeRest,
            enabled: true,
            interval_seconds,
            duration_seconds,
        }
    }
}
