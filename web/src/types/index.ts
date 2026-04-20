export interface ApiKey {
  id: string;
  name: string;
  key_hash: string;
  rate_limit: number | null;
  budget_monthly: number | null;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateKeyRequest {
  name: string;
  rate_limit?: number | null;
  budget_monthly?: number | null;
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
}

export interface Provider {
  id: string;
  name: string;
  slug: string;
  endpoints: Record<string, string> | null;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateProviderRequest {
  name: string;
  endpoints?: string | null;
}

export interface UpdateProviderRequest {
  name?: string;
  endpoints?: string | null;
  enabled?: boolean;
}

export interface Model {
  id: string;
  name: string;
  model_type?: string | null;
  pricing_policy_id?: string | null;
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

export interface UsageRecord {
  id: string;
  key_id: string;
  model_name: string;
  provider_id: string;
  channel_id: string | null;
  protocol: 'openai' | 'anthropic';
  input_tokens: number | null;
  output_tokens: number | null;
  cache_read_tokens: number | null;
  cost: number;
  created_at: string;
}

export interface UsageSummaryRecord {
  model_name: string;
  total_input_tokens: number;
  total_cache_read_tokens: number;
  total_output_tokens: number;
  total_cost: number;
  request_count: number;
}

export interface UsageFilter {
  key_id?: string;
  model_name?: string;
  since?: string;
  until?: string;
}

export interface AuditLog {
  id: string;
  key_id: string;
  model_name: string;
  provider_id: string;
  channel_id: string | null;
  protocol: 'openai' | 'anthropic';
  stream: boolean;
  request_body: string;
  response_body: string;
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
}

export interface LogFilter {
  key_id?: string;
  model_name?: string;
  since?: string;
  until?: string;
  offset?: number;
  limit?: number;
}

export interface User {
  id: string;
  username: string;
  role: 'admin' | 'user';
}

export interface LoginRequest {
  username: string;
  password: string;
}

export interface RegisterRequest {
  username: string;
  password: string;
}

export interface AuthResponse {
  token: string;
  refresh_token: string;
  user: User;
}

export interface MeResponse {
  id: string;
  username: string;
  role: 'admin' | 'user';
  allow_registration: boolean;
}

export interface AuthConfigResponse {
  allow_registration: boolean;
}

export interface RefreshResponse {
  token: string;
  refresh_token: string;
}

export interface ChangePasswordRequest {
  current_password: string;
  new_password: string;
}

export interface UserResponse {
  id: string;
  username: string;
  role: 'admin' | 'user';
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface UpdateUserRequest {
  role?: 'admin' | 'user';
  enabled?: boolean;
}

export interface SettingsResponse {
  allow_registration: boolean;
  server_host: string;
  audit_log_request: boolean;
  audit_log_response: boolean;
}

export interface UpdateSettingsRequest {
  allow_registration?: boolean;
  server_host?: string;
  audit_log_request?: boolean;
  audit_log_response?: boolean;
}

export interface Channel {
  id: string;
  provider_id: string;
  name: string;
  api_key: string;
  priority: number;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateChannelRequest {
  provider_id: string;
  name: string;
  api_key: string;
  priority?: number;
}

export interface UpdateChannelRequest {
  name?: string;
  // api_key intentionally omitted — use dedicated updateChannelApiKey
  // base_url removed — use provider.endpoints["default"]
  priority?: number;
  enabled?: boolean;
}

export interface UpdateChannelApiKeyRequest {
  api_key: string;
}

// --- Channel Models ---

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
  name: string;
  billing_type: string;
  config: Record<string, unknown>;
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
  config: Record<string, unknown>;
}

export interface UpdatePricingPolicy {
  name?: string;
  billing_type?: string;
  config?: Record<string, unknown>;
}

export interface SyncModelsResponse {
  new: number;
  updated: number;
  models: SyncedModel[];
}
