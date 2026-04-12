import { useQuery } from '@tanstack/react-query';
import { queryUsage } from '../api/usage';
import type { UsageFilter } from '../types';

export function useUsage(filter: UsageFilter) {
  return useQuery({ queryKey: ['usage', filter], queryFn: () => queryUsage(filter) });
}
