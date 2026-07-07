import { useQuery } from '@tanstack/react-query';
import { queryLogs, getLog } from '../api/logs';
import type { LogFilter } from '../types';
import { useAuthStore } from '../stores/authStore';

export function useLogs(filter: LogFilter, page = 1, pageSize = 20) {
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useQuery({
    queryKey: [slug, 'logs', filter, page, pageSize],
    queryFn: () => queryLogs(filter, page, pageSize),
    enabled: !!slug,
  });
}

export function useLog(id: string | null) {
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useQuery({
    queryKey: [slug, 'log', id],
    queryFn: () => getLog(id!),
    enabled: !!slug && !!id,
  });
}
