import { adminApiClient } from './client';
import type { Provider, CreateProviderRequest, UpdateProviderRequest, Channel, CreateChannelRequest, UpdateChannelRequest, SyncModelsResponse, ChannelModel, CreateChannelModelRequest, UpdateChannelModelRequest } from '../types';

export async function getSeedData(): Promise<{ providers: Array<{ name: string; base_url?: string; endpoints?: Record<string, string>; enabled?: boolean }>; models: Array<{ provider: string; name: string; billing_type?: string; input_price?: number; output_price?: number }> }> {
  const { data } = await adminApiClient.get('/seed');
  return data;
}

export async function listProviders(): Promise<Provider[]> {
  const { data } = await adminApiClient.get<Provider[]>('/providers');
  return data;
}

export async function getProvider(id: string): Promise<Provider> {
  const { data } = await adminApiClient.get<Provider>(`/providers/${id}`);
  return data;
}

export async function createProvider(input: CreateProviderRequest): Promise<Provider> {
  const { data } = await adminApiClient.post<Provider>('/providers', input);
  return data;
}

export async function updateProvider(id: string, input: UpdateProviderRequest): Promise<Provider> {
  const { data } = await adminApiClient.patch<Provider>(`/providers/${id}`, input);
  return data;
}

export async function deleteProvider(id: string): Promise<void> {
  await adminApiClient.delete(`/providers/${id}`);
}

export async function listAllChannels(): Promise<Channel[]> {
  const { data } = await adminApiClient.get<Channel[]>('/channels');
  return data;
}

export async function listChannels(providerId: string): Promise<Channel[]> {
  const { data } = await adminApiClient.get<Channel[]>(`/providers/${providerId}/channels`);
  return data;
}

export async function createChannel(input: CreateChannelRequest): Promise<Channel> {
  const { data } = await adminApiClient.post<Channel>('/channels', input);
  return data;
}

export async function updateChannel(id: string, input: UpdateChannelRequest): Promise<Channel> {
  const { data } = await adminApiClient.patch<Channel>(`/channels/${id}`, input);
  return data;
}

export async function getChannel(id: string): Promise<Channel> {
  const { data } = await adminApiClient.get<Channel>(`/channels/${id}`);
  return data;
}

export async function deleteChannel(id: string): Promise<void> {
  await adminApiClient.delete(`/channels/${id}`);
}

export async function syncModels(providerId: string): Promise<SyncModelsResponse> {
  const { data } = await adminApiClient.post<SyncModelsResponse>(`/providers/${providerId}/sync-models`, {});
  return data;
}

// --- Channel Models ---

export async function listChannelModels(providerId: string): Promise<ChannelModel[]> {
  const { data } = await adminApiClient.get<ChannelModel[]>(`/providers/${providerId}/channel-models`);
  return data;
}

export async function listChannelModelsByChannel(channelId: string): Promise<ChannelModel[]> {
  const { data } = await adminApiClient.get<ChannelModel[]>(`/channels/${channelId}/channel-models`);
  return data;
}

export async function createChannelModel(providerId: string, input: CreateChannelModelRequest): Promise<ChannelModel> {
  const { data } = await adminApiClient.post<ChannelModel>(`/providers/${providerId}/channel-models`, input);
  return data;
}

export async function createChannelModelByChannel(channelId: string, input: CreateChannelModelRequest): Promise<ChannelModel> {
  const { data } = await adminApiClient.post<ChannelModel>(`/channels/${channelId}/channel-models`, input);
  return data;
}

export async function getChannelModel(id: string): Promise<ChannelModel> {
  const { data } = await adminApiClient.get<ChannelModel>(`/channel-models/${id}`);
  return data;
}

export async function updateChannelModel(id: string, input: UpdateChannelModelRequest): Promise<ChannelModel> {
  const { data } = await adminApiClient.patch<ChannelModel>(`/channel-models/${id}`, input);
  return data;
}

export async function deleteChannelModel(id: string): Promise<void> {
  await adminApiClient.delete(`/channel-models/${id}`);
}
