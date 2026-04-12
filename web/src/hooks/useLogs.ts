import { useQuery } from '@tanstack/react-query';
import { queryLogs } from '../api/logs';
import type { LogFilter } from '../types';

export function useLogs(filter: LogFilter, page = 1, pageSize = 20) {
  return useQuery({ queryKey: ['logs', filter, page, pageSize], queryFn: () => queryLogs(filter, page, pageSize) });
}
