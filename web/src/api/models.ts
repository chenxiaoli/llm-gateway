import { apiClient } from './client';
import type { Model, ModelWithProvider, CreateModelRequest, CreateGlobalModelRequest, UpdateModelRequest } from '../types';

export async function listModels(providerId: string): Promise<Model[]> {
  const { data } = await apiClient.get<Model[]>(`/providers/${providerId}/models`);
  return data;
}

export async function listAllModels(): Promise<ModelWithProvider[]> {
  const { data } = await apiClient.get<ModelWithProvider[]>('/models');
  return data;
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
