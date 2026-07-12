import { adminApiClient } from './client';
import type { SettingsResponse, UpdateSettingsRequest, SystemInfo, NatsStatusResponse } from '../types';

// All routes here are platform-scoped (top-level /admin/* — no org prefix).
export async function getSettings(): Promise<SettingsResponse> {
  const { data } = await adminApiClient.get<SettingsResponse>('/settings');
  return data;
}

export async function updateSettings(input: UpdateSettingsRequest): Promise<SettingsResponse> {
  const { data } = await adminApiClient.patch<SettingsResponse>('/settings', input);
  return data;
}

export async function getSystemInfo(): Promise<SystemInfo> {
  const { data } = await adminApiClient.get<SystemInfo>('/system-info');
  return data;
}

export async function getNatsStatus(): Promise<NatsStatusResponse> {
  const { data } = await adminApiClient.get<NatsStatusResponse>('/nats/status');
  return data;
}
