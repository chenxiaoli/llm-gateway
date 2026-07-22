export interface ApiKey {
  id: string;
  name: string;
  key_hash: string;
  key_prefix: string | null;
  rate_limit: number | null;
  budget_monthly: number | null;
  enabled: boolean;
  model_fallback_id: string | null;
  auto_route_id: string | null;
  created_at: string;
  updated_at: string;
  /** Phase 7: current UTC-month MTD spend in 10^8 subunits per USD.
   *  `0` when the key has no usage this month. */
  mtd_units: number;
}

export interface CreateKeyRequest {
  name: string;
  rate_limit?: number | null;
  budget_monthly?: number | null;
  model_fallback_id?: string | null;
  auto_route_id?: string | null;
}

export interface CreateKeyResponse {
  id: string;
  name: string;
  key: string;
  rate_limit: number | null;
  budget_monthly: number | null;
  enabled: boolean;
  created_at: string;
}

export interface UpdateKeyRequest {
  name?: string;
  rate_limit?: number | null;
  budget_monthly?: number | null;
  enabled?: boolean;
  model_fallback_id?: string | null;
  auto_route_id?: string | null;
}

export interface Provider {
  id: string;
  owner_org_id: string | null;
  name: string;
  slug: string;
  endpoints: Record<string, string> | null;
  proxy_url: string | null;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateProviderRequest {
  name: string;
  endpoints?: Record<string, string | null> | null;
  proxy_url?: string | null;
}

export interface UpdateProviderRequest {
  name?: string;
  endpoints?: Record<string, string | null> | null;
  proxy_url?: string | null;
  enabled?: boolean;
}

export interface Model {
  id: string;
  owner_org_id: string | null;
  name: string;
  model_type?: string | null;
  pricing_policy_id?: string | null;
  supports_vision: boolean;
  supports_tools: boolean;
  created_at: string;
}

export interface ModelWithProvider extends Model {
  pricing_policy_name?: string | null;
  channel_ids: string[];
  channel_names: string[];
}

export interface CreateModelRequest {
  name: string;
  pricing_policy_id?: string | null;
}

export interface CreateGlobalModelRequest {
  name: string;
  pricing_policy_id?: string | null;
}

export interface UpdateModelRequest {
  pricing_policy_id?: string | null;
}

export interface UserPricingInfo {
  billing_type: string;
  config: PricingConfig;
}

export interface UserModelView {
  name: string;
  model_type: string | null;
  pricing_policy_name: string | null;
  pricing: UserPricingInfo | null;
  is_available: boolean;
  created_at: string;
}

export interface UsageRecord {
  id: string;
  request_id: string | null;
  key_id: string;
  model_name: string;
  provider_id: string;
  channel_id: string | null;
  protocol: 'openai' | 'anthropic';
  input_tokens: number | null;
  output_tokens: number | null;
  cache_read_tokens: number | null;
  cache_creation_tokens: number | null;
  cost: number;
  pricing_policy: Record<string, unknown> | null;
  weighted_tokens: number;
  created_at: string;
}

export interface UsageSummaryRecord {
  model_name: string;
  total_input_tokens: number;
  total_cache_read_tokens: number;
  total_cache_creation_tokens: number;
  total_output_tokens: number;
  total_cost: number;
  request_count: number;
}

export interface DailyUsageRecord {
  date: string;
  total_input_tokens: number;
  total_output_tokens: number;
  total_cache_read_tokens: number;
  total_cache_creation_tokens: number;
  total_weighted_tokens: number;
  total_cost: number;
  request_count: number;
}

export interface ChannelUsageSummaryRecord {
  channel_id: string | null;
  channel_name: string | null;
  total_requests: number;
  total_cost: number;
  total_input_tokens: number;
  total_output_tokens: number;
}

export interface UsageFilter {
  key_id?: string;
  user_id?: string;
  model_name?: string;
  since?: string;
  until?: string;
  tz?: string;
}

export interface RouteAttempt {
  model: string;
  channel_id: string;
  channel_name: string | null;
  status_code: number;
  error_message: string | null;
  latency_ms: number;
  started_at: string;
}

export interface AuditLogSummary {
  id: string;
  request_id: string | null;
  key_id: string;
  model_name: string;
  provider_id: string;
  channel_id: string | null;
  channel_name: string | null;
  protocol: 'openai' | 'anthropic';
  stream: boolean;
  status_code: number;
  latency_ms: number;
  input_tokens: number | null;
  output_tokens: number | null;
  created_at: string;
  original_model?: string;
  upstream_model?: string;
  model_override_reason?: string;
  request_path?: string;
  upstream_url?: string;
  request_headers?: string;
  response_headers?: string;
  routes?: RouteAttempt[] | null;
}

export interface AuditLog extends AuditLogSummary {
  request_body: string;
  response_body: string;
}

export interface LogFilter {
  request_id?: string;
  key_id?: string;
  channel_id?: string;
  model_name?: string;
  since?: string;
  until?: string;
  offset?: number;
  limit?: number;
}

export interface User {
  id: string;
  username: string | null;
  platform_role: 'platform_admin' | null;
  balance?: number;
  threshold?: number;
  /**
   * User-chosen friendly display name. null/empty when unset (user hasn't
   * visited /profile yet). When set, takes priority over username and email
   * in displayName().
   */
  nickname?: string | null;
  /**
   * User's email address. null until the user sets one (Phase 4 email flow:
   * registration collects it, but legacy accounts may have none yet).
   */
  email: string | null;
  /**
   * ISO timestamp of when the user verified their email, or null if unverified.
   */
  email_verified_at: string | null;
}

// --- Org / multi-tenant ---

export interface OrgSummary {
  id: string;
  slug: string;
  name: string;
  role: 'owner' | 'admin' | 'member';
  group_id: string | null;
}

// --- Members ---

export type MemberRole = 'owner' | 'admin' | 'member';

export interface Member {
  user_id: string;
  username: string | null;
  email: string | null;
  nickname?: string | null;
  role: MemberRole;
  group_id: string | null;
  group_name: string | null;
  enabled: boolean;
  balance: number; // USD float
  threshold: number; // USD float
  created_at: string; // ISO timestamp (was: joined_at)
}

// --- Groups ---

export interface Group {
  id: string;
  name: string;
  description: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateGroupRequest {
  name: string;
  description?: string;
}

export interface UpdateGroupRequest {
  name?: string;
  description?: string | null;
}

export interface DeleteGroupResult {
  cleared_users: number;
  cleared_channels: number;
}

export interface LoginRequest {
  username: string;
  password: string;
}

export interface RegisterRequest {
  password: string;
  /**
   * Required email address (Phase 4). A verification email is sent on
   * register; the user must click through before they can log in.
   */
  email: string;
  /**
   * Optional invitation token — when set, the backend accepts the invitation
   * server-side in the same register transaction (Task 8) instead of forcing
   * a separate client-side /invitations/accept round-trip.
   */
  inviteToken?: string;
}

export interface AuthResponse {
  token: string;
  refresh_token: string;
  user: User;
  /**
   * null for limbo users (just registered, no org yet). The post-auth flow
   * treats null here as "show the onboarding wizard".
   */
  current_org: OrgSummary | null;
  orgs: OrgSummary[];
}

export interface MeResponse {
  id: string;
  username: string | null;
  platform_role: 'platform_admin' | null;
  /**
   * null when the user has no memberships (e.g. just self-left their last
   * org). The auth store treats null as "no current org" and route guards
   * bounce the user to /login.
   */
  current_org: OrgSummary | null;
  orgs: OrgSummary[];
  allow_registration: boolean;
  /**
   * True when the current membership is a temp/system-created row, indicating
   * a platform_admin is operating in an org they don't really belong to. The
   * UI surfaces an "platform admin mode" banner when this is set.
   */
  impersonating: boolean;
  /**
   * The signed-in user's email (may be null for legacy accounts that predate
   * the email-required signup flow). Surfaced in the account UI.
   */
  email: string | null;
  /**
   * ISO timestamp of email verification, or null if the user hasn't verified
   * yet. The login flow gates unverified users (403 email_not_verified).
   */
  email_verified_at: string | null;
  /**
   * The signed-in user's chosen friendly name (mirrors `User.nickname`).
   * null/undefined when unset. Backend serializes via `skip_serializing_if =
   * Option::is_none`, so the field is absent on the wire when not set — keep
   * this optional on the client side too.
   */
  nickname?: string | null;
  /**
   * True when this server requires email verification before login. The
   * frontend surfaces UI hints (e.g. an "Add email" banner) based on this.
   */
  requires_email_verification: boolean;
}

export interface AuthConfigResponse {
  allow_registration: boolean;
  currency: string;
}

export interface RefreshResponse {
  token: string;
  refresh_token: string;
}

export interface ChangePasswordRequest {
  current_password: string;
  new_password: string;
}

export interface SettingsResponse {
  allow_registration: boolean;
  server_host: string;
  audit_log_request: boolean;
  audit_log_response: boolean;
  currency: string;
}

export interface UpdateSettingsRequest {
  allow_registration?: boolean;
  server_host?: string;
  audit_log_request?: boolean;
  audit_log_response?: boolean;
  currency?: string;
}

export interface SystemInfo {
  server_bind_address: string;
  database_driver: string;
  rate_limit_window_secs: number;
  rate_limit_flush_interval_secs: number;
  upstream_timeout_secs: number;
  audit_retention_days: number | null;
}

export interface NatsStreamInfo {
  name: string;
  messages: number;
  bytes: number;
  consumer_count: number;
  first_sequence: number;
  last_sequence: number;
  max_messages: number;
  max_age_secs: number;
  pending_messages: number;
}

export interface NatsStatusResponse {
  streams: NatsStreamInfo[];
}

export interface TimeSlot {
  days: string[];
  start: string;
  end: string;
}

export interface Channel {
  id: string;
  provider_id: string;
  name: string;
  api_key: string;
  priority: number;
  pricing_policy_id?: string | null;
  markup_ratio?: number;
  rpm_limit?: number | null;
  tpm_limit?: number | null;
  balance?: number | null;
  weight?: number | null;
  enabled: boolean;
  available_hours?: TimeSlot[] | null;
  created_by?: string | null;
  group_id?: string | null;
  group_name?: string | null;
  disabled_until?: string | null;
  created_at: string;
  updated_at: string;
  models?: ChannelModelInfo[];
}

export interface ChannelModelInfo {
  id: string;
  model_id: string;
  model_name: string;
  upstream_model_name: string | null;
  priority_override: number | null;
  pricing_policy_id: string | null;
  markup_ratio: number;
  enabled: boolean;
}

export interface CreateChannelRequest {
  provider_id: string;
  name: string;
  api_key: string;
  priority?: number;
  weight?: number | null;
  enabled?: boolean;
  available_hours?: TimeSlot[];
  models?: CreateChannelModelRequest[];
  group_id?: string | null;
}

export interface UpdateChannelRequest {
  name?: string;
  // api_key intentionally omitted — use dedicated updateChannelApiKey
  // base_url removed — use provider.endpoints["default"]
  priority?: number;
  weight?: number | null;
  enabled?: boolean;
  available_hours?: TimeSlot[];
  group_id?: string | null;
}

export interface UpdateChannelApiKeyRequest {
  api_key: string;
}

export interface ChannelTestResult {
  success: boolean;
  latency_ms: number;
  model: string;
  endpoint_key: string;
  error: string | null;
  response_data: string | null;
}

// ── Model Fallback Types ──────────────────────────────────────────────────

export interface ModelFallbackGroup {
  models: string[];
  priorities: number[];
}

export interface ModelFallbackConfig {
  id: string;
  name: string;
  config: ModelFallbackGroup[];
  created_by: string | null;
  created_at: string;
}

export interface CreateModelFallbackRequest {
  name: string;
  config: ModelFallbackGroup[];
}

export interface UpdateModelFallbackRequest {
  name?: string;
  config?: ModelFallbackGroup[];
}

// ── Auto Route Config Types ───────────────────────────────────────────────

export interface AutoRouteConfigData {
  model_names: string[];
}

export interface AutoRouteConfig {
  id: string;
  name: string;
  config: AutoRouteConfigData;
  created_by: string | null;
  created_at: string;
}

export interface CreateAutoRouteConfigRequest {
  name: string;
  config: AutoRouteConfigData;
}

export interface UpdateAutoRouteConfigRequest {
  name?: string;
  config?: AutoRouteConfigData;
}

// ── Pricing Config Types ───────────────────────────────────────────────────────
// Must match the Rust structs in crates/storage/src/types.rs exactly.
// Keys: input_price_1m, output_price_1m, cache_read_price_1m (per 1M tokens).

export interface PerTokenConfig {
  input_price_1m?: number;
  output_price_1m?: number;
  cache_read_price_1m?: number;
  cache_creation_price_1m?: number;
}

export interface PerRequestConfig {
  request_price?: number;
}

export interface PerCharacterConfig {
  input_price_1m?: number;
  output_price_1m?: number;
}

export interface TierConfig {
  up_to: number | null;
  input_price_1m: number;
  output_price_1m: number;
}

export interface TieredTokenConfig {
  tiers: TierConfig[];
}

export interface HybridConfig {
  base_per_call?: number;
  input_price_1m?: number;
  output_price_1m?: number;
  cache_read_price_1m?: number;
  cache_creation_price_1m?: number;
}

export interface ContextTier {
  up_to: number | null;
  input_price_1m?: number;
  output_price_1m?: number;
  cache_read_price_1m?: number;
  cache_creation_price_1m?: number;
}

export interface ContextTieredTokenConfig {
  tiers: ContextTier[];
}

export type PricingConfig =
  | PerTokenConfig
  | PerRequestConfig
  | PerCharacterConfig
  | TieredTokenConfig
  | HybridConfig
  | ContextTieredTokenConfig;

export interface ChannelModel {
  id: string;
  channel_id: string;
  model_id: string;
  upstream_model_name: string | null;
  priority_override: number | null;
  pricing_policy_id?: string | null;
  markup_ratio: number;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface ProviderModelInfo {
  model_id: string;
  model_name: string;
  upstream_name: string | null;
  pricing_policy_id: string | null;
}

export interface CreateChannelModelRequest {
  model_id: string;
  upstream_model_name?: string | null;
  priority_override?: number | null;
  pricing_policy_id?: string | null;
  markup_ratio?: number;
  enabled?: boolean;
}

export interface UpdateChannelModelRequest {
  upstream_model_name?: string | null;
  priority_override?: number | null;
  pricing_policy_id?: string | null;
  markup_ratio?: number;
  enabled?: boolean;
}

export interface PaginatedResponse<T> {
  items: T[];
  total: number;
  page: number;
  page_size: number;
}

export interface SyncedModel {
  name: string;
  model_type: string | null;
  created: boolean;
}

export interface PricingPolicy {
  id: string;
  owner_org_id: string | null;
  name: string;
  billing_type: string;
  config: PricingConfig;
  created_at: string;
  updated_at: string;
}

export interface PricingPolicyWithCounts extends PricingPolicy {
  model_count: number;
  channel_model_count: number;
}

export interface CreatePricingPolicy {
  name: string;
  billing_type: string;
  config: PricingConfig;
}

export interface UpdatePricingPolicy {
  name?: string;
  billing_type?: string;
  config?: PricingConfig;
}

export interface SyncModelsResponse {
  new: number;
  updated: number;
  models: SyncedModel[];
}

// ── Account & Transaction Types ───────────────────────────────────────────────

export interface Account {
  id: string;
  user_id: string;
  balance: number;
  threshold: number;
  created_at: string;
  updated_at: string;
}

export interface Transaction {
  id: string;
  account_id: string;
  type: 'credit' | 'debit' | 'credit_adjustment' | 'debit_refund';
  amount: number;
  balance_after: number;
  description: string | null;
  reference_id: string | null;
  request_id: string | null;
  created_at: string;
}

export interface AccountBalanceResponse {
  account: Account;
  transactions: PaginatedResponse<Transaction>;
}

export interface MeBalanceResponse {
  balance: number;
  threshold: number;
  transactions: PaginatedResponse<Transaction>;
}

export interface CreateTransactionRequest {
  type: 'credit' | 'credit_adjustment' | 'debit_refund';
  amount: number;
  description?: string;
  reference_id?: string;
}

export interface UpdateThresholdRequest {
  threshold: number;
}

// ── Request Details (transaction drill-down) ──────────────────────────────

export interface RequestTransaction {
  id: string;
  account_id: string;
  type: string;
  amount: number;
  balance_after: number;
  description: string | null;
  reference_id: string | null;
  request_id: string | null;
  created_at: string;
}

export interface RequestUsage {
  id: string;
  request_id: string;
  key_id: string;
  model_name: string;
  provider_id: string;
  channel_id: string | null;
  protocol: string;
  input_tokens: number | null;
  output_tokens: number | null;
  cache_read_tokens: number | null;
  cache_creation_tokens: number | null;
  cost: number;
  created_at: string;
}

export interface RequestAudit {
  id: string;
  request_id: string | null;
  key_id: string;
  model_name: string;
  provider_id: string;
  channel_id: string | null;
  channel_name: string | null;
  protocol: string;
  stream: boolean;
  request_body: string;
  response_body: string;
  status_code: number;
  latency_ms: number;
  input_tokens: number | null;
  output_tokens: number | null;
  created_at: string;
  original_model: string | null;
  upstream_model: string | null;
  model_override_reason: string | null;
  request_path: string | null;
  upstream_url: string | null;
  request_headers: string | null;
  response_headers: string | null;
}

export interface RequestDetailsResponse {
  transaction: RequestTransaction | null;
  usage: RequestUsage | null;
  audit: RequestAudit | null;
}

// Phase 3: invitations
export interface Invitation {
  id: string;
  token: string;
  url: string;
  role: 'member' | 'admin';
  /**
   * Phase 4: the email this invitation was sent to. Optional only for legacy
   * rows; new rows always carry a recipient per `create_invitation`'s required
   * body field.
   */
  recipient_email: string | null;
  created_at: string;
  expires_at: string;
  accepted_at: string | null;
  accepted_by: string | null; // username, not user id
  revoked_at: string | null;
}

export interface InvitationPreview {
  org_name: string;
  org_slug: string;
  role: 'member' | 'admin';
  inviter_username: string | null;
  /**
   * Phase 4: the email the admin bound this invitation to. The landing page
   * surfaces it so the recipient can confirm the address matches their account.
   */
  recipient_email: string;
  expires_at: string;
}

export interface CreateInvitationBody {
  role: 'member' | 'admin';
  /**
   * Phase 4: required. The invitation is bound to this recipient email; only a
   * verified user with a matching email can accept it.
   */
  recipient_email: string;
}

export interface AcceptInvitationBody {
  token: string;
}
