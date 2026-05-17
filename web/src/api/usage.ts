import { apiClient } from './client';
import type { PaginatedResponse, UsageFilter, UsageRecord, UsageSummaryRecord, ChannelUsageSummaryRecord, DailyUsageRecord } from '../types';

export async function queryUsage(filter: UsageFilter = {}, page = 1, pageSize = 20): Promise<PaginatedResponse<UsageRecord>> {
  const params: Record<string, string | number> = { page, page_size: pageSize };
  if (filter.key_id) params.key_id = filter.key_id;
  if (filter.user_id) params.user_id = filter.user_id;
  if (filter.model_name) params.model_name = filter.model_name;
  if (filter.since) params.since = filter.since;
  if (filter.until) params.until = filter.until;
  const { data } = await apiClient.get<PaginatedResponse<UsageRecord>>('/usage', { params });
  return data;
}

export async function queryUsageSummary(filter: UsageFilter = {}): Promise<UsageSummaryRecord[]> {
  const params: Record<string, string> = {};
  if (filter.key_id) params.key_id = filter.key_id;
  if (filter.user_id) params.user_id = filter.user_id;
  if (filter.model_name) params.model_name = filter.model_name;
  if (filter.since) params.since = filter.since;
  if (filter.until) params.until = filter.until;
  const { data } = await apiClient.get<UsageSummaryRecord[]>('/usage/summary', { params });
  return data;
}

export async function queryChannelUsageSummary(filter: UsageFilter = {}): Promise<ChannelUsageSummaryRecord[]> {
  const params: Record<string, string> = {};
  if (filter.key_id) params.key_id = filter.key_id;
  if (filter.user_id) params.user_id = filter.user_id;
  if (filter.model_name) params.model_name = filter.model_name;
  if (filter.since) params.since = filter.since;
  if (filter.until) params.until = filter.until;
  const { data } = await apiClient.get<ChannelUsageSummaryRecord[]>('/usage/channel-summary', { params });
  return data;
}

export async function queryDailyUsage(filter: UsageFilter): Promise<DailyUsageRecord[]> {
  const params = new URLSearchParams();
  if (filter.key_id) params.set('key_id', filter.key_id);
  if (filter.user_id) params.set('user_id', filter.user_id);
  if (filter.model_name) params.set('model_name', filter.model_name);
  if (filter.since) params.set('since', filter.since);
  if (filter.until) params.set('until', filter.until);
  const query = params.toString() ? `?${params.toString()}` : '';
  const { data } = await apiClient.get<DailyUsageRecord[]>(`/usage/daily${query}`);
  return data;
}
