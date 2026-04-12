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
  api_key: string;
  openai_base_url: string | null;
  anthropic_base_url: string | null;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateProviderRequest {
  name: string;
  api_key: string;
  openai_base_url?: string | null;
  anthropic_base_url?: string | null;
}

export interface UpdateProviderRequest {
  name?: string;
  api_key?: string;
  openai_base_url?: string | null;
  anthropic_base_url?: string | null;
  enabled?: boolean;
}

export interface Model {
  name: string;
  provider_id: string;
  billing_type: 'token' | 'request';
  input_price: number;
  output_price: number;
  request_price: number;
  enabled: boolean;
  created_at: string;
}

export interface CreateModelRequest {
  name: string;
  billing_type: 'token' | 'request';
  input_price?: number;
  output_price?: number;
  request_price?: number;
}

export interface UpdateModelRequest {
  billing_type?: 'token' | 'request';
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
  protocol: 'openai' | 'anthropic';
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
