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
  base_url: string | null;
  endpoints: string | null;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateProviderRequest {
  name: string;
  base_url?: string | null;
  endpoints?: string | null;
}

export interface UpdateProviderRequest {
  name?: string;
  base_url?: string | null;
  endpoints?: string | null;
  enabled?: boolean;
}

export interface Model {
  id: string;
  name: string;
  provider_id: string;
  billing_type: string;
  pricing_policy_id?: string | null;
  input_price: number;
  output_price: number;
  request_price: number;
  enabled: boolean;
  created_at: string;
}

export interface ModelWithProvider extends Model {
  provider_name: string;
}

export interface CreateModelRequest {
  name: string;
  billing_type: string;
  input_price?: number;
  output_price?: number;
  request_price?: number;
}

export interface CreateGlobalModelRequest {
  provider_id: string;
  name: string;
  billing_type: string;
  input_price?: number;
  output_price?: number;
  request_price?: number;
  enabled?: boolean;
}

export interface UpdateModelRequest {
  billing_type?: string;
  input_price?: number;
  output_price?: number;
  request_price?: number;
  enabled?: boolean;
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
  cost: number;
  created_at: string;
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
  base_url: string | null;
  priority: number;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateChannelRequest {
  provider_id: string;
  name: string;
  api_key: string;
  base_url?: string | null;
  priority?: number;
}

export interface UpdateChannelRequest {
  name?: string;
  api_key?: string;
  base_url?: string | null;
  priority?: number;
  enabled?: boolean;
}

// --- Channel Models ---

export interface ChannelModel {
  id: string;
  channel_id: string;
  model_id: string;
  upstream_model_name: string;
  priority_override: number | null;
  billing_type?: string | null;
  input_price?: number | null;
  output_price?: number | null;
  request_price?: number | null;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateChannelModelRequest {
  model_id: string;
  upstream_model_name: string;
  priority_override?: number | null;
  billing_type?: string | null;
  input_price?: number | null;
  output_price?: number | null;
  request_price?: number | null;
  enabled?: boolean;
}

export interface UpdateChannelModelRequest {
  upstream_model_name?: string;
  priority_override?: number | null;
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

export interface SyncModelsResponse {
  new: number;
  updated: number;
  models: SyncedModel[];
}
