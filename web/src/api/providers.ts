import { apiClient, orgPrefix } from './client';
import type { Provider, CreateProviderRequest, UpdateProviderRequest, Channel, CreateChannelRequest, UpdateChannelRequest, ChannelModel, CreateChannelModelRequest, UpdateChannelModelRequest, UpdateChannelApiKeyRequest, ProviderModelInfo, ChannelTestResult } from '../types';

export async function getSeedData(): Promise<{ providers: Array<{ name: string; endpoints?: Record<string, string>; enabled?: boolean }>; models: Array<{ provider: string; name: string; billing_type?: string; input_price?: number; output_price?: number }> }> {
  const { data } = await apiClient.get(`${orgPrefix()}/admin/seed`);
  return data;
}

export async function listProviders(): Promise<Provider[]> {
  const { data } = await apiClient.get<Provider[]>(`${orgPrefix()}/admin/providers`);
  return data;
}

export async function getProvider(id: string): Promise<Provider> {
  const { data } = await apiClient.get<Provider>(`${orgPrefix()}/admin/providers/${id}`);
  return data;
}

export async function createProvider(input: CreateProviderRequest): Promise<Provider> {
  const { data } = await apiClient.post<Provider>(`${orgPrefix()}/admin/providers`, input);
  return data;
}

export async function updateProvider(id: string, input: UpdateProviderRequest): Promise<Provider> {
  const { data } = await apiClient.patch<Provider>(`${orgPrefix()}/admin/providers/${id}`, input);
  return data;
}

export async function deleteProvider(id: string): Promise<void> {
  await apiClient.delete(`${orgPrefix()}/admin/providers/${id}`);
}

export async function listAllChannels(): Promise<Channel[]> {
  const { data } = await apiClient.get<Channel[]>(`${orgPrefix()}/admin/channels`);
  return data;
}

export async function listChannels(providerId: string): Promise<Channel[]> {
  const { data } = await apiClient.get<Channel[]>(`${orgPrefix()}/admin/providers/${providerId}/channels`);
  return data;
}

export async function createChannel(input: CreateChannelRequest): Promise<Channel> {
  const { data } = await apiClient.post<Channel>(`${orgPrefix()}/admin/channels`, input);
  return data;
}

export async function updateChannel(id: string, input: UpdateChannelRequest): Promise<Channel> {
  const { data } = await apiClient.patch<Channel>(`${orgPrefix()}/admin/channels/${id}`, input);
  return data;
}

export async function updateChannelApiKey(id: string, input: UpdateChannelApiKeyRequest): Promise<Channel> {
  const { data } = await apiClient.patch<Channel>(`${orgPrefix()}/admin/channels/${id}/api-key`, input);
  return data;
}

export async function getChannel(id: string): Promise<Channel> {
  const { data } = await apiClient.get<Channel>(`${orgPrefix()}/admin/channels/${id}`);
  return data;
}

export async function deleteChannel(id: string): Promise<void> {
  await apiClient.delete(`${orgPrefix()}/admin/channels/${id}`);
}

export async function testChannel(id: string, endpointKey?: string, stream?: boolean): Promise<ChannelTestResult[]> {
  const params = new URLSearchParams();
  if (endpointKey) params.set('endpoint_key', endpointKey);
  if (stream !== undefined) params.set('stream', String(stream));
  const query = params.toString() ? `?${params.toString()}` : '';
  const { data } = await apiClient.post<ChannelTestResult[]>(`${orgPrefix()}/admin/channels/${id}/test${query}`);
  return data;
}

export async function listProviderModels(providerId: string): Promise<ProviderModelInfo[]> {
  const { data } = await apiClient.get<ProviderModelInfo[]>(`${orgPrefix()}/admin/providers/${providerId}/models`);
  return data;
}

export async function updateProviderModels(providerId: string, models: { model_id: string; upstream_name?: string; pricing_policy_id?: string | null }[]): Promise<ProviderModelInfo[]> {
  const { data } = await apiClient.put<ProviderModelInfo[]>(`${orgPrefix()}/admin/providers/${providerId}/models`, { models });
  return data;
}

// --- Channel Models ---

export async function listChannelModels(providerId: string): Promise<ChannelModel[]> {
  const { data } = await apiClient.get<ChannelModel[]>(`${orgPrefix()}/admin/providers/${providerId}/channel-models`);
  return data;
}

export async function listChannelModelsByChannel(channelId: string): Promise<ChannelModel[]> {
  const { data } = await apiClient.get<ChannelModel[]>(`${orgPrefix()}/admin/channels/${channelId}/channel-models`);
  return data;
}

export async function createChannelModel(providerId: string, input: CreateChannelModelRequest): Promise<ChannelModel> {
  const { data } = await apiClient.post<ChannelModel>(`${orgPrefix()}/admin/providers/${providerId}/channel-models`, input);
  return data;
}

export async function createChannelModelByChannel(channelId: string, input: CreateChannelModelRequest): Promise<ChannelModel> {
  const { data } = await apiClient.post<ChannelModel>(`${orgPrefix()}/admin/channels/${channelId}/channel-models`, input);
  return data;
}

export async function getChannelModel(id: string): Promise<ChannelModel> {
  const { data } = await apiClient.get<ChannelModel>(`${orgPrefix()}/admin/channel-models/${id}`);
  return data;
}

export async function updateChannelModel(id: string, input: UpdateChannelModelRequest): Promise<ChannelModel> {
  const { data } = await apiClient.patch<ChannelModel>(`${orgPrefix()}/admin/channel-models/${id}`, input);
  return data;
}

export async function deleteChannelModel(id: string): Promise<void> {
  await apiClient.delete(`${orgPrefix()}/admin/channel-models/${id}`);
}
