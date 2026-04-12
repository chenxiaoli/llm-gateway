import { apiClient } from './client';
import type { ApiKey, CreateKeyRequest, CreateKeyResponse, UpdateKeyRequest } from '../types';

export async function listKeys(): Promise<ApiKey[]> {
  const { data } = await apiClient.get<ApiKey[]>('/keys');
  return data;
}

export async function getKey(id: string): Promise<ApiKey> {
  const { data } = await apiClient.get<ApiKey>(`/keys/${id}`);
  return data;
}

export async function createKey(input: CreateKeyRequest): Promise<CreateKeyResponse> {
  const { data } = await apiClient.post<CreateKeyResponse>('/keys', input);
  return data;
}

export async function updateKey(id: string, input: UpdateKeyRequest): Promise<ApiKey> {
  const { data } = await apiClient.patch<ApiKey>(`/keys/${id}`, input);
  return data;
}

export async function deleteKey(id: string): Promise<void> {
  await apiClient.delete(`/keys/${id}`);
}
