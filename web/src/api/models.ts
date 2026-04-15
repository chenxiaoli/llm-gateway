import { apiClient } from './client';
import type { Model, ModelWithProvider, CreateModelRequest, CreateGlobalModelRequest, UpdateModelRequest } from '../types';

export async function listModels(providerId: string): Promise<Model[]> {
  const { data } = await apiClient.get<Model[]>(`/providers/${providerId}/models`);
  return data;
}

export async function listAllModels(): Promise<ModelWithProvider[]> {
  const { data } = await apiClient.get<Array<{
    model: Model;
    provider_name: string;
    billing_type: 'token' | 'request';
    openai_compatible: boolean;
    anthropic_compatible: boolean;
  }>>('/models');
  return data.map(item => ({
    id: item.model.id,
    name: item.model.name,
    provider_id: item.model.provider_id,
    billing_type: item.billing_type,
    pricing_policy_id: item.model.pricing_policy_id,
    input_price: item.model.input_price,
    output_price: item.model.output_price,
    request_price: item.model.request_price,
    enabled: item.model.enabled,
    created_at: item.model.created_at,
    provider_name: item.provider_name,
  }));
}

export async function createModel(providerId: string, input: CreateModelRequest): Promise<Model> {
  const { data } = await apiClient.post<Model>(`/providers/${providerId}/models`, input);
  return data;
}

export async function createGlobalModel(input: CreateGlobalModelRequest): Promise<Model> {
  const { data } = await apiClient.post<Model>('/models', input);
  return data;
}

export async function updateModel(providerId: string, modelName: string, input: UpdateModelRequest): Promise<Model> {
  const { data } = await apiClient.patch<Model>(`/providers/${providerId}/models/${modelName}`, input);
  return data;
}

export async function deleteModel(providerId: string, modelName: string): Promise<void> {
  await apiClient.delete(`/providers/${providerId}/models/${modelName}`);
}
