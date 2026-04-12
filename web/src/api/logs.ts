import { apiClient } from './client';
import type { AuditLog, LogFilter } from '../types';

export async function queryLogs(filter: LogFilter = {}): Promise<AuditLog[]> {
  const params: Record<string, string | number> = {};
  if (filter.key_id) params.key_id = filter.key_id;
  if (filter.model_name) params.model_name = filter.model_name;
  if (filter.since) params.since = filter.since;
  if (filter.until) params.until = filter.until;
  if (filter.offset != null) params.offset = filter.offset;
  if (filter.limit != null) params.limit = filter.limit;
  const { data } = await apiClient.get<AuditLog[]>('/logs', { params });
  return data;
}
