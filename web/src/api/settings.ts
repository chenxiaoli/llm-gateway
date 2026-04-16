import { adminApiClient } from './client';
import type { SettingsResponse, UpdateSettingsRequest } from '../types';

export async function getSettings(): Promise<SettingsResponse> {
  const { data } = await adminApiClient.get<SettingsResponse>('/settings');
  return data;
}

export async function updateSettings(input: UpdateSettingsRequest): Promise<SettingsResponse> {
  const { data } = await adminApiClient.patch<SettingsResponse>('/settings', input);
  return data;
}
