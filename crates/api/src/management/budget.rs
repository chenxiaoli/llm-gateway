//! Phase 7: read-only budget observability endpoints.
//!
//! `GET /api/v1/{org_slug}/budget-status` returns the org's current UTC-month
//! accrued spend in 10^8 subunits per USD, plus the `YYYY-MM` month bucket so
//! the frontend can display it. The org-wide default *budget* value is NOT
//! returned here — the frontend composes it from `GET /{slug}/defaults` (one
//! source of truth per datum). Read-only: no write endpoints in this phase.

use axum::extract::State;
use axum::Json;
use chrono::{Datelike, Utc};
use serde::Serialize;
use std::sync::Arc;

use crate::error::ApiError;
use crate::AppState;
use llm_gateway_org::OrgContext;

/// Response body for `GET /api/v1/{org_slug}/budget-status`.
#[derive(Debug, Serialize)]
pub struct BudgetStatusResponse {
    /// Month-to-date spend in 10^8 subunits per USD. `0` when the org has no
    /// usage this month. Frontend converts to USD at the rendering boundary
    /// via the existing `unitsToUsd` helper.
    pub accrued_units: i64,
    /// UTC calendar month in `YYYY-MM` form. Matches the bucket Phase 6's
    /// `budget_counters` rows are keyed by.
    pub month_bucket: String,
}

/// GET /api/v1/{org_slug}/budget-status — read org-wide MTD spend.
///
/// Membership is enforced upstream by `membership_layer` before this handler
/// runs (same gate as `GET /{slug}/defaults` in Phase 5). The handler itself
/// is a thin read: one indexed SUM, one bucket string.
pub async fn get_budget_status(
    State(state): State<Arc<AppState>>,
    ctx: OrgContext,
) -> Result<Json<BudgetStatusResponse>, ApiError> {
    let accrued_units = state
        .storage
        .get_org_month_to_date_spend(&ctx.org_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    let now = Utc::now();
    let month_bucket = format!("{:04}-{:02}", now.year(), now.month());

    Ok(Json(BudgetStatusResponse {
        accrued_units,
        month_bucket,
    }))
}
