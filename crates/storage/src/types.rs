use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// --- Org / Membership ---

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemberRole {
    Owner,
    Admin,
    Member,
}

impl MemberRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            MemberRole::Owner => "owner",
            MemberRole::Admin => "admin",
            MemberRole::Member => "member",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "owner" => MemberRole::Owner,
            "admin" => MemberRole::Admin,
            "member" => MemberRole::Member,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformRole {
    PlatformAdmin,
}

impl PlatformRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            PlatformRole::PlatformAdmin => "platform_admin",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "platform_admin" => Some(PlatformRole::PlatformAdmin),
            _ => None,
        }
    }
}

#[derive(Debug, Error)]
pub enum SetPlatformRoleError {
    #[error("user not found")]
    UserNotFound,
    #[error("cannot demote the last platform admin")]
    LastPlatformAdmin,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Org {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub owner_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CreateOrg {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub owner_id: String,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct UpdateOrg {
    pub name: Option<String>,
    pub slug: Option<String>,
}

/// Org-wide default settings surfaced via `GET/PUT /api/v1/orgs/{id}/defaults`.
///
/// Stored as two rows in `org_settings` kv table; this struct is the typed
/// facade. `default_budget_monthly_usd` is in USD subunits (10⁸ per USD,
/// i.e. `UNITS_PER_USD` in `crates/storage/src/money.rs`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrgDefaults {
    /// Default per-key RPM cap for keys whose own `rate_limit` is NULL.
    /// None = unlimited.
    pub default_rate_limit_rpm: Option<i64>,
    /// Default per-key monthly budget in USD subunits (10⁸ per USD). None = no budget.
    /// NOTE: stored for display; NOT enforced in Phase 5.
    pub default_budget_monthly_usd: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Member {
    pub user_id: String,
    pub org_id: String,
    pub role: MemberRole,
    pub group_id: Option<String>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Default per-membership account threshold in subunits (1.00 USD at 6-decimal
/// precision, i.e. 10⁸ per USD). Matches the migration default in
/// 20260426000000_accounts_and_transactions.sql. Used when inserting new
/// account rows (upsert_member / accept_invitation) and as the COALESCE
/// fallback when joining accounts in `list_members`.
pub const DEFAULT_ACCOUNT_THRESHOLD_SUBUNITS: i64 = 100_000_000;

/// A membership row joined with its user, optional group, and per-membership
/// account. This is the shape `list_members` returns so the Members page UI
/// can render balance / enabled / email / group_name columns in one round-trip
/// (no N+1 of `get_user` / `get_account`).
///
/// `role` is the raw lowercase string from `members.role` (matches
/// `MemberRole::as_str`). `balance` / `threshold` are subunits (10⁸ per USD);
/// when no `accounts` row exists the SQL COALESCEs them to `0` and
/// `DEFAULT_ACCOUNT_THRESHOLD_SUBUNITS` respectively.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct MemberWithDetails {
    pub user_id: String,
    pub org_id: String,
    pub username: String,
    pub email: Option<String>,
    /// Raw lowercase role string from `members.role` (`"owner" | "admin" | "member"`).
    pub role: String,
    pub group_id: Option<String>,
    pub group_name: Option<String>,
    /// From `users.enabled`.
    pub enabled: bool,
    /// From `accounts.balance` (subunits, 10⁸ per USD). 0 when no account row.
    pub balance: i64,
    /// From `accounts.threshold` (subunits, 10⁸ per USD). Falls back to
    /// `DEFAULT_ACCOUNT_THRESHOLD_SUBUNITS` when no account row.
    pub threshold: i64,
    /// From `members.created_at`.
    pub created_at: DateTime<Utc>,
}

/// Summary of a user's membership in one org.
///
/// NOTE for Task 5: this struct has a nested `org: Org` field. The SQL for
/// `list_orgs_for_user` will need either a custom `FromRow` impl that
/// constructs `Org` from the prefixed columns, or the query should use
/// `o.id AS "org.id"` etc. with `#[sqlx(flatten)]` if supported.
/// Plain `query_as::<_, MembershipSummary>` will fail.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MembershipSummary {
    pub org: Org,
    pub role: MemberRole,
    pub group_id: Option<String>,
}

// --- Invitations (Phase 3) ---

/// One row of the `invitations` table. Used internally by the storage trait.
///
/// Phase 4: `recipient_email` binds the invitation to a specific email. Old
/// generic-token rows (Phase 3) had NULL here; the migration revokes them.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Invitation {
    pub id: String,
    pub token: String,
    pub org_id: String,
    pub role: MemberRole,
    pub created_by: String,
    pub recipient_email: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub accepted_by: Option<String>,
    pub revoked_at: Option<DateTime<Utc>>,
}

/// One row of the `email_verifications` table. The `token` is the lookup
/// key the user clicks in their email; `consumed_at IS NULL` means "still
/// usable". `email` is denormalized from `users.email` so the row records
/// *what* was being verified at mint time.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EmailVerification {
    pub id: String,
    pub token: String,
    pub user_id: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub consumed_at: Option<DateTime<Utc>>,
}

/// One row of the `password_resets` table. Same shape as
/// `EmailVerification` minus the denormalized `email`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PasswordReset {
    pub id: String,
    pub token: String,
    pub user_id: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub consumed_at: Option<DateTime<Utc>>,
}

/// Result of an atomic password-reset attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordResetOutcome {
    /// Token was valid; password was updated; reset row was marked consumed.
    Success,
    /// No row matched the token.
    NotFound,
    /// Token had already been consumed.
    Consumed,
    /// Token had expired.
    Expired,
}

/// Response for invitation mint/list endpoints. The URL is constructed
/// server-side so the frontend doesn't need to know the public base URL.
///
/// Phase 4: `recipient_email` is surfaced so the admin list can show whom the
/// invitation was sent to (Task 11). It's Option only for legacy rows — new
/// rows always carry a recipient per `create_invitation`'s required body field.
#[derive(Debug, Clone, serde::Serialize)]
pub struct InvitationResponse {
    pub id: String,
    pub token: String,
    pub url: String,
    pub role: String,
    pub recipient_email: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub accepted_by: Option<String>,
    pub revoked_at: Option<DateTime<Utc>>,
}

/// Request body for `POST /api/v1/invitations/accept`.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct AcceptInvitationRequest {
    pub token: String,
}

/// Response for `GET /api/v1/invitations/preview?token=...`. Public — does
/// NOT include the token itself (the caller already has it) or any user-id
/// data; just enough for the landing page to render.
///
/// Note: `already_member` was originally specced but is computed client-side
/// (the frontend already has the user's membership list) — drop it from the
/// type per the plan's self-review note.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InvitationPreview {
    pub org_name: String,
    pub org_slug: String,
    pub role: String,
    pub inviter_username: String,
    pub recipient_email: Option<String>,
    pub expires_at: DateTime<Utc>,
}

// --- Pagination ---

#[derive(Debug, Clone, Serialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    #[serde(default, deserialize_with = "deserialize_i64_opt")]
    pub page: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_i64_opt")]
    pub page_size: Option<i64>,
}

fn deserialize_i64_opt<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<i64>, D::Error> {
    let s: Option<String> = Option::deserialize(d)?;
    match s {
        None => Ok(None),
        Some(v) => v.parse::<i64>().map(Some).map_err(serde::de::Error::custom),
    }
}

impl PaginationParams {
    pub fn normalized(&self) -> (i64, i64) {
        let page = self.page.unwrap_or(1).max(1);
        let page_size = self.page_size.unwrap_or(20).clamp(1, 100);
        (page, page_size)
    }
}

// --- API Keys ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub org_id: String,
    pub name: String,
    pub key_hash: String,
    pub key_prefix: Option<String>,
    pub rate_limit: Option<i64>,       // global RPM, None = unlimited
    pub budget_monthly: Option<i64>,   // monthly budget cap, None = unlimited
    pub enabled: bool,
    pub created_by: Option<String>,
    pub model_fallback_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// ApiKey + its current UTC-month MTD spend. The MTD field is `0` when the
/// key has no budget_counters row this month (mirrors SQL `COALESCE(..., 0)`).
/// Used by the Phase 7 keys-listing endpoint so the UI can render per-key
/// spend in one round-trip without an N+1 of `get_month_to_date_spend`.
#[derive(Debug, Clone, Serialize)]
pub struct ApiKeyWithMtd {
    pub key: ApiKey,
    pub mtd_units: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateApiKey {
    pub org_id: String,
    pub name: String,
    pub rate_limit: Option<i64>,
    pub budget_monthly: Option<i64>,
    pub model_fallback_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateApiKey {
    pub name: Option<String>,
    pub rate_limit: Option<Option<i64>>,
    pub budget_monthly: Option<Option<i64>>,
    pub enabled: Option<bool>,
    pub model_fallback_id: Option<Option<String>>,
}

// --- Providers ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: String,
    pub owner_org_id: Option<String>,
    pub name: String,
    pub slug: String,
    pub endpoints: Option<String>,      // JSON string {"default": "...", "openai": "...", "anthropic": "..."}
    pub proxy_url: Option<String>,      // HTTP/SOCKS proxy URL, e.g. "http://user:pass@proxy:8080"
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Provider with endpoints parsed as JSON object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderWithEndpoints {
    pub id: String,
    pub owner_org_id: Option<String>,
    pub name: String,
    pub slug: String,
    pub endpoints: Option<std::collections::HashMap<String, String>>,
    pub proxy_url: Option<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Provider> for ProviderWithEndpoints {
    fn from(p: Provider) -> Self {
        let endpoints = p.endpoints.and_then(|e| serde_json::from_str(&e).ok());
        ProviderWithEndpoints {
            id: p.id,
            owner_org_id: p.owner_org_id,
            name: p.name,
            slug: p.slug,
            endpoints,
            proxy_url: p.proxy_url,
            enabled: p.enabled,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateProvider {
    pub owner_org_id: Option<String>,
    pub name: String,
    pub slug: Option<String>,
    pub endpoints: Option<serde_json::Value>,
    pub proxy_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProvider {
    pub name: Option<String>,
    pub endpoints: Option<Option<serde_json::Value>>,
    pub proxy_url: Option<Option<String>>,
    pub enabled: Option<bool>,
}

// --- Provider Models ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderModel {
    pub provider_id: String,
    pub model_id: String,
    pub owner_org_id: Option<String>,
    pub upstream_name: Option<String>,
    pub pricing_policy_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderModelInfo {
    pub model_id: String,
    pub model_name: String,
    pub upstream_name: Option<String>,
    pub pricing_policy_id: Option<String>,
}

// --- Channels ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSlot {
    pub days: Vec<String>,
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub org_id: String,
    pub provider_id: String,
    pub name: String,
    pub api_key: String,
    pub priority: i32,
    pub pricing_policy_id: Option<String>,
    pub markup_ratio: i64,
    pub rpm_limit: Option<i64>,
    pub tpm_limit: Option<i64>,
    pub balance: Option<i64>,
    pub weight: Option<i32>,
    pub enabled: bool,
    pub disabled_until: Option<DateTime<Utc>>,
    pub available_hours: Option<Vec<TimeSlot>>,
    pub created_by: Option<String>,
    pub group_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateChannel {
    pub org_id: String,
    pub provider_id: String,
    pub name: String,
    pub api_key: String,
    pub priority: Option<i32>,
    pub pricing_policy_id: Option<String>,
    pub markup_ratio: Option<i64>,
    pub rpm_limit: Option<i64>,
    pub tpm_limit: Option<i64>,
    pub balance: Option<i64>,
    pub weight: Option<i32>,
    pub enabled: Option<bool>,
    pub available_hours: Option<Vec<TimeSlot>>,
    pub models: Option<Vec<CreateChannelModel>>,
    pub group_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateChannel {
    pub name: Option<String>,
    // api_key intentionally omitted — use dedicated /api-key endpoint
    // base_url removed — use provider.endpoints["default"] for fallback
    pub priority: Option<i32>,
    pub pricing_policy_id: Option<Option<String>>,
    pub markup_ratio: Option<i64>,
    pub enabled: Option<bool>,
    pub rpm_limit: Option<Option<i64>>,
    pub tpm_limit: Option<Option<i64>>,
    pub balance: Option<Option<i64>>,
    pub weight: Option<Option<i32>>,
    pub available_hours: Option<Vec<TimeSlot>>,
    pub group_id: Option<Option<String>>,
}

/// Dedicated payload for updating a channel's API key.
/// Using a separate type and endpoint prevents accidental key clearing
/// when updating other channel fields.
#[derive(Debug, Deserialize)]
pub struct UpdateChannelApiKey {
    pub api_key: String,
}

// --- Models ---

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Model {
    pub id: String,           // primary key
    pub owner_org_id: Option<String>,
    pub name: String,          // display name
    pub model_type: Option<String>,
    pub pricing_policy_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

// Deprecated: kept for migration compatibility only
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BillingType {
    Token,
    Request,
}

// --- Pricing Policies ---

/// Per-token pricing config: prices in integer subunits (100M per USD) per 1M tokens.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct PerTokenConfig {
    /// Input token price per 1M tokens in subunits (e.g. 300_000_000 = $3.00/M).
    pub input_price_1m: Option<i64>,
    /// Output token price per 1M tokens in subunits.
    pub output_price_1m: Option<i64>,
    /// Cache read price per 1M tokens in subunits (cheaper than input).
    pub cache_read_price_1m: Option<i64>,
    /// Cache creation price per 1M tokens in subunits.
    pub cache_creation_price_1m: Option<i64>,
}

impl PerTokenConfig {
    pub fn input_price(&self) -> i64 { self.input_price_1m.unwrap_or(0).max(0) }
    pub fn output_price(&self) -> i64 { self.output_price_1m.unwrap_or(0).max(0) }
    pub fn cache_read_price(&self) -> i64 { self.cache_read_price_1m.unwrap_or(0).max(0) }
    pub fn cache_creation_price(&self) -> i64 { self.cache_creation_price_1m.unwrap_or(0).max(0) }
    pub fn divisor(&self) -> i64 { 1_000_000i64 }
}

/// Per-request pricing config: flat fee per API call.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PerRequestConfig {
    pub request_price: Option<i64>,
}

impl PerRequestConfig {
    pub fn price_per_call(&self) -> i64 { self.request_price.unwrap_or(0).max(0) }
}

/// Per-character pricing config: prices in $ per 1M characters.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PerCharacterConfig {
    pub input_price_1m: Option<i64>,
    pub output_price_1m: Option<i64>,
}

impl PerCharacterConfig {
    pub fn input_price(&self) -> i64 { self.input_price_1m.unwrap_or(0).max(0) }
    pub fn output_price(&self) -> i64 { self.output_price_1m.unwrap_or(0).max(0) }
    pub fn divisor(&self) -> i64 { 1_000_000i64 }
}

/// Single tier for tiered token pricing.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TierConfig {
    /// Upper bound of this tier in tokens (null = final tier).
    pub up_to: Option<i64>,
    pub input_price_1m: Option<i64>,
    pub output_price_1m: Option<i64>,
}

impl TierConfig {
    pub fn input_price(&self) -> i64 { self.input_price_1m.unwrap_or(0).max(0) }
    pub fn output_price(&self) -> i64 { self.output_price_1m.unwrap_or(0).max(0) }
    pub fn divisor(&self) -> i64 {
        if self.input_price_1m.is_some() || self.output_price_1m.is_some() {
            1_000_000i64
        } else {
            1_000i64
        }
    }
}

/// Tiered token pricing config.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TieredTokenConfig {
    pub tiers: Vec<TierConfig>,
}

impl TieredTokenConfig {
    pub fn tier_divisor(&self) -> i64 { 1_000_000i64 }
}

/// Single tier for context-tiered (threshold-based) token pricing.
/// ALL tokens in a request use the price of the matched tier.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextTier {
    /// Exclusive upper bound of input token count. null = final tier (no limit).
    pub up_to: Option<i64>,
    pub input_price_1m: Option<i64>,
    pub output_price_1m: Option<i64>,
    pub cache_read_price_1m: Option<i64>,
    pub cache_creation_price_1m: Option<i64>,
}

impl ContextTier {
    pub fn input_price(&self) -> i64 { self.input_price_1m.unwrap_or(0).max(0) }
    pub fn output_price(&self) -> i64 { self.output_price_1m.unwrap_or(0).max(0) }
    pub fn cache_read_price(&self) -> i64 { self.cache_read_price_1m.unwrap_or(0).max(0) }
    pub fn cache_creation_price(&self) -> i64 { self.cache_creation_price_1m.unwrap_or(0).max(0) }
}

/// Context-tiered (threshold-based) token pricing config.
/// The tier is determined by total input token count; all tokens use that tier's prices.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextTieredTokenConfig {
    pub tiers: Vec<ContextTier>,
}

impl ContextTieredTokenConfig {
    pub fn divisor(&self) -> i64 { 1_000_000i64 }
}

/// Hybrid pricing config: base fee per call + per-token.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HybridConfig {
    pub base_per_call: Option<i64>,
    pub input_price_1m: Option<i64>,
    pub output_price_1m: Option<i64>,
    pub cache_read_price_1m: Option<i64>,
    pub cache_creation_price_1m: Option<i64>,
}

impl HybridConfig {
    pub fn input_price(&self) -> i64 { self.input_price_1m.unwrap_or(0).max(0) }
    pub fn output_price(&self) -> i64 { self.output_price_1m.unwrap_or(0).max(0) }
    pub fn cache_read_price(&self) -> i64 { self.cache_read_price_1m.unwrap_or(0).max(0) }
    pub fn cache_creation_price(&self) -> i64 { self.cache_creation_price_1m.unwrap_or(0).max(0) }
    pub fn divisor(&self) -> i64 { 1_000_000i64 }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingPolicy {
    pub id: String,
    pub owner_org_id: Option<String>,
    pub name: String,
    pub billing_type: String,
    pub config: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingPolicyWithCounts {
    #[serde(flatten)]
    pub policy: PricingPolicy,
    pub model_count: i64,
    pub channel_model_count: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreatePricingPolicy {
    pub owner_org_id: Option<String>,
    pub name: String,
    pub billing_type: String,
    pub config: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePricingPolicy {
    pub name: Option<String>,
    pub billing_type: Option<String>,
    pub config: Option<serde_json::Value>,
}

// --- Usage for pricing calculation ---

#[derive(Debug, Clone, Copy)]
pub struct Usage {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub input_chars: Option<i64>,
    pub output_chars: Option<i64>,
    pub request_count: i64,
    pub cache_read_tokens: Option<i64>, // tokens read from cache (cheaper)
    pub cache_creation_tokens: Option<i64>, // tokens written to cache
}

impl Usage {
    pub fn from_tokens(input: Option<i64>, output: Option<i64>, requests: i64) -> Self {
        Usage {
            input_tokens: input.unwrap_or(0),
            output_tokens: output.unwrap_or(0),
            input_chars: None,
            output_chars: None,
            request_count: requests,
            cache_read_tokens: None,
            cache_creation_tokens: None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateModel {
    pub owner_org_id: Option<String>,
    pub name: String,
    pub pricing_policy_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateModel {
    pub pricing_policy_id: Option<Option<String>>,  // None=keep, Some(None)=clear
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelWithProvider {
    pub model: Model,
    pub pricing_policy_name: Option<String>,
    pub channel_ids: Vec<String>,
    pub channel_names: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct UserPricingInfo {
    pub billing_type: String,
    pub config: serde_json::Value,
}

#[derive(Debug, serde::Serialize)]
pub struct UserModelView {
    pub name: String,
    pub model_type: Option<String>,
    pub pricing_policy_name: Option<String>,
    pub pricing: Option<UserPricingInfo>,
    pub is_available: bool,
    pub created_at: String,
}

// --- Channel Models (Junction Table) ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelModel {
    pub id: String,
    pub org_id: String,
    pub channel_id: String,
    pub model_id: String,
    pub upstream_model_name: Option<String>,
    pub priority_override: Option<i32>,
    pub pricing_policy_id: Option<String>,
    pub markup_ratio: i64,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, serde::Serialize)]
pub struct ChannelTestResult {
    pub success: bool,
    pub latency_ms: u64,
    pub model: String,
    pub endpoint_key: String,
    pub error: Option<String>,
    pub response_data: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateChannelModel {
    pub channel_id: Option<String>,
    pub model_id: String,
    pub upstream_model_name: Option<String>,
    pub priority_override: Option<i32>,
    pub pricing_policy_id: Option<String>,
    pub markup_ratio: Option<i64>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateChannelModel {
    pub upstream_model_name: Option<String>,
    pub priority_override: Option<Option<i32>>,
    pub pricing_policy_id: Option<Option<String>>,
    pub markup_ratio: Option<i64>,
    pub enabled: Option<bool>,
}

// --- Key-Model Rate Limits ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyModelRateLimit {
    pub org_id: String,
    pub key_id: String,
    pub model_id: String,
    pub rpm: i64,
    pub tpm: i64,
}

// --- Usage Records ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    pub id: String,
    pub org_id: String,
    pub request_id: Option<String>,
    pub key_id: String,
    pub model_name: String,
    pub provider_id: String,
    pub channel_id: Option<String>,
    pub protocol: Protocol,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cache_read_tokens: Option<i64>,
    pub cache_creation_tokens: Option<i64>,
    pub cost: i64,
    pub pricing_policy: Option<serde_json::Value>,
    pub weighted_tokens: i64,
    pub user_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    Openai,
    Anthropic,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageSummaryRecord {
    pub model_name: String,
    pub total_input_tokens: i64,
    pub total_cache_read_tokens: i64,
    pub total_cache_creation_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cost: i64,
    pub request_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChannelUsageSummaryRecord {
    pub channel_id: Option<String>,
    pub channel_name: Option<String>,
    pub total_requests: i64,
    pub total_cost: i64,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct DailyUsageRecord {
    pub date: String,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cache_read_tokens: i64,
    pub total_cache_creation_tokens: i64,
    pub total_weighted_tokens: i64,
    pub total_cost: i64,
    pub request_count: i64,
}

fn deserialize_datetime_opt<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<DateTime<Utc>>, D::Error> {
    let s: Option<String> = Option::deserialize(d)?;
    match s {
        None => Ok(None),
        Some(v) => DateTime::parse_from_rfc3339(&v)
            .map(|dt| Some(dt.with_timezone(&Utc)))
            .map_err(serde::de::Error::custom),
    }
}

#[derive(Debug, Deserialize)]
pub struct UsageFilter {
    pub key_id: Option<String>,
    pub user_id: Option<String>,
    pub model_name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_datetime_opt")]
    pub since: Option<DateTime<Utc>>,
    #[serde(default, deserialize_with = "deserialize_datetime_opt")]
    pub until: Option<DateTime<Utc>>,
    #[serde(default)]
    pub tz: Option<String>,
}

/// One upstream routing attempt captured during proxy failover.
/// Each entry is one try of the client request against a specific
/// (channel, channel_model) combination. Failed attempts record the
/// status and error; successful attempts record None for error_message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteAttempt {
    /// The channel model that was actually used for this attempt
    /// (may differ from the client's original model due to channel mapping).
    pub model: String,
    pub channel_id: String,
    pub channel_name: Option<String>,
    /// Provider of the channel for this attempt. Empty for pseudo entries
    /// (routing miss, no channels available) where no channel was reached.
    pub provider_id: String,
    /// 0 = connection error (no HTTP response received).
    /// Otherwise the upstream HTTP status code.
    pub status_code: i32,
    /// None when the attempt succeeded.
    pub error_message: Option<String>,
    pub latency_ms: i64,
    pub started_at: DateTime<Utc>,
}

// --- Audit Logs ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: String,
    pub org_id: String,
    pub request_id: Option<String>,
    pub key_id: String,
    pub model_name: String,
    pub provider_id: String,
    pub channel_id: Option<String>,
    pub channel_name: Option<String>,
    pub protocol: Protocol,
    pub stream: bool,
    pub request_body: String,
    pub response_body: String,
    pub status_code: i32,
    pub latency_ms: i64,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub original_model: Option<String>,
    pub upstream_model: Option<String>,
    pub model_override_reason: Option<String>,
    pub request_path: Option<String>,
    pub upstream_url: Option<String>,
    pub request_headers: Option<String>,
    pub response_headers: Option<String>,
    pub user_id: Option<String>,
    pub actor_is_platform_admin: bool,
    /// Per-upstream-attempt history. None for legacy rows (data created
    /// before the v1.8.0 migration). New rows always populate this.
    pub routes: Option<Vec<RouteAttempt>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogSummary {
    pub id: String,
    pub request_id: Option<String>,
    pub key_id: String,
    pub model_name: String,
    pub provider_id: String,
    pub channel_id: Option<String>,
    pub channel_name: Option<String>,
    pub protocol: Protocol,
    pub stream: bool,
    pub status_code: i32,
    pub latency_ms: i64,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub original_model: Option<String>,
    pub upstream_model: Option<String>,
    pub model_override_reason: Option<String>,
    pub request_path: Option<String>,
    pub upstream_url: Option<String>,
    pub request_headers: Option<String>,
    pub response_headers: Option<String>,
    pub user_id: Option<String>,
    /// See AuditLog::routes. None for legacy rows.
    pub routes: Option<Vec<RouteAttempt>>,
}

#[derive(Debug, Deserialize)]
pub struct LogFilter {
    pub request_id: Option<String>,
    pub key_id: Option<String>,
    pub channel_id: Option<String>,
    #[serde(skip)]
    pub user_id: Option<String>,
    pub model_name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_datetime_opt")]
    pub since: Option<DateTime<Utc>>,
    #[serde(default, deserialize_with = "deserialize_datetime_opt")]
    pub until: Option<DateTime<Utc>>,
    #[serde(default, deserialize_with = "deserialize_i64_opt")]
    pub offset: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_i64_opt")]
    pub limit: Option<i64>,
}

// --- Users ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub password: String,
    pub platform_role: Option<PlatformRole>,
    pub current_org_id: Option<String>,
    pub enabled: bool,
    pub refresh_token: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // Phase 4: email + verification + password-change tracking.
    pub email: Option<String>,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub requires_email_verification: bool,
    pub password_changed_at: DateTime<Utc>,
}

// TODO(Task 5/8): migrate alongside User — drop role/group_id fields once
// list_users_paginated and the management handlers stop reading them.
// User was migrated in this commit; these sibling types were intentionally
// left for the next task to avoid expanding scope.
#[derive(Debug, Clone, Serialize)]
pub struct UserWithBalance {
    pub id: String,
    pub username: String,
    pub role: String,
    pub enabled: bool,
    pub group_id: Option<String>,
    pub group_name: Option<String>,
    pub balance: i64,
    pub threshold: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateUser {
    pub username: String,
    pub password: String,
}

// TODO(Task 5/8): migrate alongside User — drop role/group_id fields once
// list_users_paginated and the management handlers stop reading them.
// User was migrated in this commit; these sibling types were intentionally
// left for the next task to avoid expanding scope.
#[derive(Debug, Deserialize)]
pub struct UpdateUser {
    pub role: Option<String>,
    pub enabled: Option<bool>,
    pub group_id: Option<Option<String>>,
}

// --- Groups ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: String,
    pub org_id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateGroup {
    /// Ignored by handlers — `org_id` is taken from the request's OrgContext.
    /// Kept on the struct for backwards compatibility with older clients.
    #[serde(default)]
    pub org_id: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateGroup {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
}

#[derive(Debug, Serialize)]
pub struct DeleteGroupResult {
    pub cleared_users: i64,
    pub cleared_channels: i64,
}

// --- Accounts ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub org_id: String,
    pub user_id: String,
    pub balance: i64,
    pub threshold: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub org_id: String,
    pub account_id: String,
    #[serde(rename = "type")]
    pub transaction_type: TransactionType,
    pub amount: i64,
    pub balance_after: i64,
    pub description: Option<String>,
    pub reference_id: Option<String>,
    pub request_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionType {
    Credit,
    Debit,
    CreditAdjustment,
    DebitRefund,
}

impl TransactionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TransactionType::Credit => "credit",
            TransactionType::Debit => "debit",
            TransactionType::CreditAdjustment => "credit_adjustment",
            TransactionType::DebitRefund => "debit_refund",
        }
    }
}

impl std::fmt::Display for TransactionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// --- Account API Request/Response types ---

#[derive(Debug, Deserialize)]
pub struct CreateTransaction {
    #[serde(rename = "type")]
    pub transaction_type: String,
    pub amount: i64,
    pub description: Option<String>,
    pub reference_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAccountThreshold {
    pub threshold: i64,
}

/// Request to atomically deduct balance from an account.
pub struct DeductBalance {
    pub account_id: String,
    pub amount: i64,
    pub transaction_type: TransactionType,
    pub description: Option<String>,
    pub reference_id: Option<String>,
    pub request_id: Option<String>,
}

/// Result of a deduct_balance operation.
#[derive(Debug)]
pub enum DeductBalanceResult {
    Success(Transaction),
    InsufficientBalance { current_balance: i64, requested: i64 },
    AccountNotFound,
}

/// Request to atomically add balance to an account (credits, refunds).
pub struct AddBalance {
    pub account_id: String,
    pub amount: i64,
    pub transaction_type: TransactionType,
    pub description: Option<String>,
    pub reference_id: Option<String>,
}

/// Result of an add_balance operation.
#[derive(Debug)]
pub enum AddBalanceResult {
    Success(Transaction),
    AccountNotFound,
}

#[derive(Debug, Clone, Serialize)]
pub struct AccountResponse {
    pub id: String,
    pub user_id: String,
    pub balance: i64,
    pub threshold: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl From<&Account> for AccountResponse {
    fn from(a: &Account) -> Self {
        AccountResponse {
            id: a.id.clone(),
            user_id: a.user_id.clone(),
            balance: a.balance,
            threshold: a.threshold,
            created_at: a.created_at.to_rfc3339(),
            updated_at: a.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TransactionResponse {
    pub id: String,
    pub account_id: String,
    #[serde(rename = "type")]
    pub transaction_type: String,
    pub amount: i64,
    pub balance_after: i64,
    pub description: Option<String>,
    pub reference_id: Option<String>,
    pub created_at: String,
}

impl From<&Transaction> for TransactionResponse {
    fn from(t: &Transaction) -> Self {
        TransactionResponse {
            id: t.id.clone(),
            account_id: t.account_id.clone(),
            transaction_type: t.transaction_type.as_str().to_string(),
            amount: t.amount,
            balance_after: t.balance_after,
            description: t.description.clone(),
            reference_id: t.reference_id.clone(),
            created_at: t.created_at.to_rfc3339(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_type_as_str() {
        assert_eq!(TransactionType::Credit.as_str(), "credit");
        assert_eq!(TransactionType::Debit.as_str(), "debit");
        assert_eq!(TransactionType::CreditAdjustment.as_str(), "credit_adjustment");
        assert_eq!(TransactionType::DebitRefund.as_str(), "debit_refund");
    }

    #[test]
    fn test_account_response_from() {
        let now = chrono::Utc::now();
        let account = Account {
            id: "acc-1".to_string(),
            org_id: "org_test".to_string(),
            user_id: "user-1".to_string(),
            balance: 10050,
            threshold: 100,
            created_at: now,
            updated_at: now,
        };
        let response = AccountResponse::from(&account);
        assert_eq!(response.id, "acc-1");
        assert_eq!(response.balance, 10050);
    }
}

// --- Model Fallback ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelFallbackGroup {
    pub models: Vec<String>,
    pub priorities: Vec<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelFallbackConfig {
    pub id: String,
    pub name: String,
    pub config: Vec<ModelFallbackGroup>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateModelFallback {
    pub name: String,
    pub config: Vec<ModelFallbackGroup>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateModelFallback {
    pub name: Option<String>,
    pub config: Option<Vec<ModelFallbackGroup>>,
}

// --- Settings ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub allow_registration: bool,
}

// --- Config ---

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub auth: AuthConfig,
    pub database: DatabaseConfig,
    pub rate_limit: RateLimitConfig,
    pub upstream: UpstreamConfig,
    pub audit: AuditConfig,
    pub nats: Option<NatsConfig>,
    #[serde(default)]
    pub email: EmailConfig,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub encryption_key: String,
    /// Public-facing base URL the frontend is served at (no trailing slash).
    /// Used to build invitation links etc. Defaults to
    /// `http://localhost:5173` when unset.
    #[serde(default)]
    pub public_base_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub allow_registration: Option<bool>,
    /// When true (default), the first user to register against an empty DB
    /// is automatically promoted to `platform_admin`. Operators who want to
    /// bootstrap via the CLI subcommand instead should set this to false.
    /// Self-hosted deployments typically leave this on; SaaS deployments
    /// typically turn it off.
    #[serde(default = "default_first_user_is_admin")]
    pub first_user_is_admin: bool,
}

fn default_first_user_is_admin() -> bool { true }

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub driver: String,
    pub url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RateLimitConfig {
    pub flush_interval_secs: i64,
    pub window_size_secs: i64,
}

#[derive(Debug, Deserialize)]
pub struct UpstreamConfig {
    pub timeout_secs: u64,
}

#[derive(Debug, Deserialize)]
pub struct AuditConfig {
    pub retention_days: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NatsConfig {
    pub url: String,
    #[serde(default)]
    pub token: Option<String>,
    /// Path to a NATS credentials file (JWT + NKey seed) for JWT-based auth.
    /// Takes precedence over `token` when set.
    #[serde(default)]
    pub credentials_file: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EmailConfig {
    /// "smtp" | "file" | "noop"
    #[serde(default = "default_email_transport")]
    pub transport: String,
    #[serde(default = "default_email_from_address")]
    pub from_address: String,
    #[serde(default = "default_email_from_name")]
    pub from_name: String,
    /// Used when transport = "file"
    #[serde(default = "default_email_file_output_dir")]
    pub file_output_dir: String,
    /// SMTP-specific — required when transport = "smtp"
    pub smtp_host: Option<String>,
    pub smtp_port: Option<u16>,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
    #[serde(default = "default_email_smtp_use_tls")]
    pub smtp_use_tls: bool,
}

fn default_email_transport() -> String { "file".into() }
fn default_email_from_address() -> String { "noreply@example.com".into() }
fn default_email_from_name() -> String { "LLM Gateway".into() }
fn default_email_file_output_dir() -> String { "./dev-emails".into() }
fn default_email_smtp_use_tls() -> bool { true }

impl Default for EmailConfig {
    fn default() -> Self {
        Self {
            transport: default_email_transport(),
            from_address: default_email_from_address(),
            from_name: default_email_from_name(),
            file_output_dir: default_email_file_output_dir(),
            smtp_host: None,
            smtp_port: None,
            smtp_username: None,
            smtp_password: None,
            smtp_use_tls: default_email_smtp_use_tls(),
        }
    }
}
