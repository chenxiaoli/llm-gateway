import { apiClient, orgPrefix } from './client';
import type { ChannelModel, CreateChannelModelRequest, UpdateChannelModelRequest } from '../types';

export async function listChannelModelsByProvider(providerId: string): Promise<ChannelModel[]> {
  const { data } = await apiClient.get<ChannelModel[]>(`${orgPrefix()}/admin/providers/${providerId}/channel-models`);
  return data;
}

export async function createChannelModel(providerId: string, input: CreateChannelModelRequest): Promise<ChannelModel> {
  const { data } = await apiClient.post<ChannelModel>(`${orgPrefix()}/admin/providers/${providerId}/channel-models`, input);
  return data;
}

export async function updateChannelModel(id: string, input: UpdateChannelModelRequest): Promise<ChannelModel> {
  const { data } = await apiClient.patch<ChannelModel>(`${orgPrefix()}/admin/channel-models/${id}`, input);
  return data;
}

export async function deleteChannelModel(id: string): Promise<void> {
  await apiClient.delete(`${orgPrefix()}/admin/channel-models/${id}`);
}
