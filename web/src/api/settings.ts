import { apiClient, adminApiClient, orgPrefix } from './client';
import type { SettingsResponse, UpdateSettingsRequest, SystemInfo, NatsStatusResponse } from '../types';

export async function getSettings(): Promise<SettingsResponse> {
  const { data } = await apiClient.get<SettingsResponse>(`${orgPrefix()}/admin/settings`);
  return data;
}

export async function updateSettings(input: UpdateSettingsRequest): Promise<SettingsResponse> {
  const { data } = await apiClient.patch<SettingsResponse>(`${orgPrefix()}/admin/settings`, input);
  return data;
}

// Platform-level routes — not org-scoped (mounted outside management_router)
export async function getSystemInfo(): Promise<SystemInfo> {
  const { data } = await adminApiClient.get<SystemInfo>('/system-info');
  return data;
}

export async function getNatsStatus(): Promise<NatsStatusResponse> {
  const { data } = await adminApiClient.get<NatsStatusResponse>('/nats/status');
  return data;
}
