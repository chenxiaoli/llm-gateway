import { apiClient } from './client';
import type { PaginatedResponse, UsageFilter, UsageRecord } from '../types';

export async function queryUsage(filter: UsageFilter = {}, page = 1, pageSize = 20): Promise<PaginatedResponse<UsageRecord>> {
  const params: Record<string, string | number> = { page, page_size: pageSize };
  if (filter.key_id) params.key_id = filter.key_id;
  if (filter.model_name) params.model_name = filter.model_name;
  if (filter.since) params.since = filter.since;
  if (filter.until) params.until = filter.until;
  const { data } = await apiClient.get<PaginatedResponse<UsageRecord>>('/usage', { params });
  return data;
}
