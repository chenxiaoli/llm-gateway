import { adminApiClient } from './client';
import type { Model, ModelWithProvider, CreateGlobalModelRequest, UpdateModelRequest } from '../types';

export async function listAllModels(): Promise<ModelWithProvider[]> {
  const { data } = await adminApiClient.get<Array<{
    model: Model;
    pricing_policy_name: string | null;
    channel_ids: string[];
    channel_names: string[];
  }>>('/models');
  return data.map(item => ({
    id: item.model.id,
    name: item.model.name,
    model_type: item.model.model_type,
    pricing_policy_id: item.model.pricing_policy_id,
    enabled: item.model.enabled,
    created_at: item.model.created_at,
    pricing_policy_name: item.pricing_policy_name,
    channel_ids: item.channel_ids,
    channel_names: item.channel_names,
  }));
}

export async function createGlobalModel(input: CreateGlobalModelRequest): Promise<Model> {
  const { data } = await adminApiClient.post<Model>('/models', input);
  return data;
}

export async function updateModel(modelName: string, input: UpdateModelRequest): Promise<Model> {
  const { data } = await adminApiClient.patch<Model>(`/models/${modelName}`, input);
  return data;
}

export async function deleteModel(modelName: string): Promise<void> {
  await adminApiClient.delete(`/models/${modelName}`);
}
