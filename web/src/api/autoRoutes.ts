import { apiClient, orgPrefix } from './client';
import type { AutoRouteConfig, CreateAutoRouteConfigRequest, UpdateAutoRouteConfigRequest } from '../types';

export async function listAutoRouteConfigs(): Promise<AutoRouteConfig[]> {
  const { data } = await apiClient.get<AutoRouteConfig[]>(`${orgPrefix()}/auto-route-configs`);
  return data;
}

export async function getAutoRouteConfig(id: string): Promise<AutoRouteConfig> {
  const { data } = await apiClient.get<AutoRouteConfig>(`${orgPrefix()}/auto-route-configs/${id}`);
  return data;
}

export async function createAutoRouteConfig(input: CreateAutoRouteConfigRequest): Promise<AutoRouteConfig> {
  const { data } = await apiClient.post<AutoRouteConfig>(`${orgPrefix()}/auto-route-configs`, input);
  return data;
}

export async function updateAutoRouteConfig(id: string, input: UpdateAutoRouteConfigRequest): Promise<AutoRouteConfig> {
  const { data } = await apiClient.patch<AutoRouteConfig>(`${orgPrefix()}/auto-route-configs/${id}`, input);
  return data;
}

export async function deleteAutoRouteConfig(id: string): Promise<void> {
  await apiClient.delete(`${orgPrefix()}/auto-route-configs/${id}`);
}
