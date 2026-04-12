import { apiClient } from './client';
import type { ApiKey, CreateKeyRequest, CreateKeyResponse, PaginatedResponse, UpdateKeyRequest } from '../types';

export async function listKeys(page = 1, pageSize = 20): Promise<PaginatedResponse<ApiKey>> {
  const { data } = await apiClient.get<PaginatedResponse<ApiKey>>('/keys', {
    params: { page, page_size: pageSize },
  });
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
