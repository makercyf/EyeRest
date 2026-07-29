use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SuppressionReason {
    Idle,
    SessionLocked,
    FullscreenApp,
    WhitelistedProcess,
    PausedByUser,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuppressionStatus {
    pub suppressed: bool,
    pub reasons: Vec<SuppressionReason>,
}

#[derive(Debug, Clone, Default)]
pub struct SuppressionEngine {
    reasons: Arc<RwLock<BTreeSet<SuppressionReason>>>,
}

impl SuppressionEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn set_reason(&self, reason: SuppressionReason, active: bool) -> SuppressionStatus {
        let mut reasons = self.reasons.write().await;
        if active {
            reasons.insert(reason);
        } else {
            reasons.remove(&reason);
        }
        status_from_reasons(&reasons)
    }

    pub async fn status(&self) -> SuppressionStatus {
        let reasons = self.reasons.read().await;
        status_from_reasons(&reasons)
    }
}

fn status_from_reasons(reasons: &BTreeSet<SuppressionReason>) -> SuppressionStatus {
    let list = reasons.iter().cloned().collect::<Vec<_>>();
    SuppressionStatus {
        suppressed: !list.is_empty(),
        reasons: list,
    }
}
