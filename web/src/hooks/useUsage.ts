import { useQuery } from '@tanstack/react-query';
import { queryUsage } from '../api/usage';
import type { UsageFilter } from '../types';

export function useUsage(filter: UsageFilter, page = 1, pageSize = 20) {
  return useQuery({ queryKey: ['usage', filter, page, pageSize], queryFn: () => queryUsage(filter, page, pageSize) });
}
