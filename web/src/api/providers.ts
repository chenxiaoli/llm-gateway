import { apiClient } from './client';
import type { Provider, CreateProviderRequest, UpdateProviderRequest, Channel, CreateChannelRequest, UpdateChannelRequest, SyncModelsResponse, ChannelModel, CreateChannelModelRequest, UpdateChannelModelRequest } from '../types';

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

// --- Channel Models ---

export async function listChannelModels(providerId: string): Promise<ChannelModel[]> {
  const { data } = await apiClient.get<ChannelModel[]>(`/providers/${providerId}/channel-models`);
  return data;
}

export async function createChannelModel(providerId: string, input: CreateChannelModelRequest): Promise<ChannelModel> {
  const { data } = await apiClient.post<ChannelModel>(`/providers/${providerId}/channel-models`, input);
  return data;
}

export async function getChannelModel(id: string): Promise<ChannelModel> {
  const { data } = await apiClient.get<ChannelModel>(`/channel-models/${id}`);
  return data;
}

export async function updateChannelModel(id: string, input: UpdateChannelModelRequest): Promise<ChannelModel> {
  const { data } = await apiClient.patch<ChannelModel>(`/channel-models/${id}`, input);
  return data;
}

export async function deleteChannelModel(id: string): Promise<void> {
  await apiClient.delete(`/channel-models/${id}`);
}
