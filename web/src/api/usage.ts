import { apiClient } from './client';
import type { UsageRecord, UsageFilter } from '../types';

export async function queryUsage(filter: UsageFilter = {}): Promise<UsageRecord[]> {
  const params: Record<string, string> = {};
  if (filter.key_id) params.key_id = filter.key_id;
  if (filter.model_name) params.model_name = filter.model_name;
  if (filter.since) params.since = filter.since;
  if (filter.until) params.until = filter.until;
  const { data } = await apiClient.get<UsageRecord[]>('/usage', { params });
  return data;
}
