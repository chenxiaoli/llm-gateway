import { apiClient, orgPrefix } from './client';
import type { ModelFallbackConfig, CreateModelFallbackRequest, UpdateModelFallbackRequest } from '../types';

export async function listModelFallbacks(): Promise<ModelFallbackConfig[]> {
  const { data } = await apiClient.get<ModelFallbackConfig[]>(`${orgPrefix()}/model-fallbacks`);
  return data;
}

export async function getModelFallback(id: string): Promise<ModelFallbackConfig> {
  const { data } = await apiClient.get<ModelFallbackConfig>(`${orgPrefix()}/model-fallbacks/${id}`);
  return data;
}

export async function createModelFallback(input: CreateModelFallbackRequest): Promise<ModelFallbackConfig> {
  const { data } = await apiClient.post<ModelFallbackConfig>(`${orgPrefix()}/model-fallbacks`, input);
  return data;
}

export async function updateModelFallback(id: string, input: UpdateModelFallbackRequest): Promise<ModelFallbackConfig> {
  const { data } = await apiClient.patch<ModelFallbackConfig>(`${orgPrefix()}/model-fallbacks/${id}`, input);
  return data;
}

export async function deleteModelFallback(id: string): Promise<void> {
  await apiClient.delete(`${orgPrefix()}/model-fallbacks/${id}`);
}
