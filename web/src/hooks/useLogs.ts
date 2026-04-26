import { useQuery } from '@tanstack/react-query';
import { queryLogs, getLog } from '../api/logs';
import type { LogFilter } from '../types';

export function useLogs(filter: LogFilter, page = 1, pageSize = 20) {
  return useQuery({ queryKey: ['logs', filter, page, pageSize], queryFn: () => queryLogs(filter, page, pageSize) });
}

export function useLog(id: string | null) {
  return useQuery({ queryKey: ['log', id], queryFn: () => getLog(id!), enabled: !!id });
}
