import { adminApiClient } from './client';
import type { AuditLog, LogFilter, PaginatedResponse } from '../types';

export async function queryLogs(filter: LogFilter = {}, page = 1, pageSize = 20): Promise<PaginatedResponse<AuditLog>> {
  const params: Record<string, string | number> = { page, page_size: pageSize };
  if (filter.key_id) params.key_id = filter.key_id;
  if (filter.model_name) params.model_name = filter.model_name;
  if (filter.since) params.since = filter.since;
  if (filter.until) params.until = filter.until;
  const { data } = await adminApiClient.get<PaginatedResponse<AuditLog>>('/logs', { params });
  return data;
}
