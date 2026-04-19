import { adminApiClient } from './client';
import type { ChannelModel, CreateChannelModelRequest, UpdateChannelModelRequest } from '../types';

export async function listChannelModelsByProvider(providerId: string): Promise<ChannelModel[]> {
  const { data } = await adminApiClient.get<ChannelModel[]>(`/providers/${providerId}/channel-models`);
  return data;
}

export async function createChannelModel(providerId: string, input: CreateChannelModelRequest): Promise<ChannelModel> {
  const { data } = await adminApiClient.post<ChannelModel>(`/providers/${providerId}/channel-models`, input);
  return data;
}

export async function updateChannelModel(id: string, input: UpdateChannelModelRequest): Promise<ChannelModel> {
  const { data } = await adminApiClient.patch<ChannelModel>(`/channel-models/${id}`, input);
  return data;
}

export async function deleteChannelModel(id: string): Promise<void> {
  await adminApiClient.delete(`/channel-models/${id}`);
}
