import { apiClient } from './client';
import type { Provider, CreateProviderRequest, UpdateProviderRequest, Channel, CreateChannelRequest, UpdateChannelRequest, SyncModelsResponse } from '../types';

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

export async function listChannels(providerId: string): Promise<Channel[]> {
  const { data } = await apiClient.get<Channel[]>(`/providers/${providerId}/channels`);
  return data;
}

export async function createChannel(providerId: string, input: CreateChannelRequest): Promise<Channel> {
  const { data } = await apiClient.post<Channel>(`/providers/${providerId}/channels`, input);
  return data;
}

export async function updateChannel(id: string, input: UpdateChannelRequest): Promise<Channel> {
  const { data } = await apiClient.patch<Channel>(`/channels/${id}`, input);
  return data;
}

export async function deleteChannel(id: string): Promise<void> {
  await apiClient.delete(`/channels/${id}`);
}

export async function syncModels(providerId: string): Promise<SyncModelsResponse> {
  const { data } = await apiClient.post<SyncModelsResponse>(`/providers/${providerId}/sync-models`, {});
  return data;
}
