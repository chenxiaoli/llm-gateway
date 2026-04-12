import { apiClient } from './client';
import type { Provider, CreateProviderRequest, UpdateProviderRequest } from '../types';

export async function listProviders(): Promise<Provider[]> {
  const { data } = await apiClient.get<Provider[]>('/providers');
  return data;
}

export async function getProvider(id: string): Promise<Provider> {
  const { data } = await apiClient.get<Provider>(`/providers/${id}`);
  return data;
}

export async function createProvider(input: CreateProviderRequest): Promise<Provider> {
  const { data } = await apiClient.post<Provider>('/providers', input);
  return data;
}

export async function updateProvider(id: string, input: UpdateProviderRequest): Promise<Provider> {
  const { data } = await apiClient.patch<Provider>(`/providers/${id}`, input);
  return data;
}

export async function deleteProvider(id: string): Promise<void> {
  await apiClient.delete(`/providers/${id}`);
}
