import { apiClient } from './client';
import type { Model, CreateModelRequest, UpdateModelRequest } from '../types';

export async function createModel(providerId: string, input: CreateModelRequest): Promise<Model> {
  const { data } = await apiClient.post<Model>(`/providers/${providerId}/models`, input);
  return data;
}

export async function updateModel(providerId: string, modelName: string, input: UpdateModelRequest): Promise<Model> {
  const { data } = await apiClient.patch<Model>(`/providers/${providerId}/models/${modelName}`, input);
  return data;
}

export async function deleteModel(providerId: string, modelName: string): Promise<void> {
  await apiClient.delete(`/providers/${providerId}/models/${modelName}`);
}
