import { apiClient, orgPrefix } from './client';
import type { Model, ModelWithProvider, CreateGlobalModelRequest, UpdateModelRequest } from '../types';

export async function listAllModels(): Promise<ModelWithProvider[]> {
  const { data } = await apiClient.get<Array<{
    model: Model;
    pricing_policy_name: string | null;
    channel_ids: string[];
    channel_names: string[];
  }>>(`${orgPrefix()}/admin/models`);
  return data.map(item => ({
    id: item.model.id,
    name: item.model.name,
    model_type: item.model.model_type,
    pricing_policy_id: item.model.pricing_policy_id,
    created_at: item.model.created_at,
    pricing_policy_name: item.pricing_policy_name,
    channel_ids: item.channel_ids,
    channel_names: item.channel_names,
  }));
}

export async function createGlobalModel(input: CreateGlobalModelRequest): Promise<Model> {
  const { data } = await apiClient.post<Model>(`${orgPrefix()}/admin/models`, input);
  return data;
}

export async function updateModel(modelName: string, input: UpdateModelRequest): Promise<Model> {
  const { data } = await apiClient.patch<Model>(`${orgPrefix()}/admin/models/${modelName}`, input);
  return data;
}

export async function deleteModel(modelName: string): Promise<void> {
  await apiClient.delete(`${orgPrefix()}/admin/models/${modelName}`);
}
