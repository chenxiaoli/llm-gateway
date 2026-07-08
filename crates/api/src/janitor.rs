//! Background janitor task that reaps stale platform-admin impersonation
//! member rows.
//!
//! Temp rows are created by `membership_layer` when a platform_admin visits
//! an org they don't belong to. They're useful while the admin is actively
//! debugging, but they accumulate forever without this safety net. The
//! janitor deletes rows older than the cutoff (default 1 hour) based on
//! `members.last_seen`.

use std::sync::Arc;
use chrono::{DateTime, Duration, Utc};
use crate::AppState;

/// Delete temp member rows whose `last_seen` is older than `older_than`
/// ago. Returns the count of deleted rows for logging.
pub async fn cleanup_stale_impersonations(
    state: &Arc<AppState>,
    older_than: Duration,
) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
    let cutoff: DateTime<Utc> = Utc::now() - older_than;
    state.storage.delete_stale_impersonations(cutoff).await
}
