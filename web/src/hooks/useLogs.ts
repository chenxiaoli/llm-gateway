import { useQuery } from '@tanstack/react-query';
import { queryLogs } from '../api/logs';
import type { LogFilter } from '../types';

export function useLogs(filter: LogFilter) {
  return useQuery({ queryKey: ['logs', filter], queryFn: () => queryLogs(filter) });
}
