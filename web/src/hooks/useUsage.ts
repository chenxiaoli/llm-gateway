import { useQuery } from '@tanstack/react-query';
import { queryUsage, queryUsageSummary, queryChannelUsageSummary, queryDailyUsage } from '../api/usage';
import type { UsageFilter } from '../types';
import { useAuthStore } from '../stores/authStore';

export function useUsage(filter: UsageFilter, page = 1, pageSize = 20) {
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useQuery({
    queryKey: [slug, 'usage', filter, page, pageSize],
    queryFn: () => queryUsage(filter, page, pageSize),
    enabled: !!slug,
  });
}

export function useUsageSummary(filter: UsageFilter) {
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useQuery({
    queryKey: [slug, 'usage-summary', filter],
    queryFn: () => queryUsageSummary(filter),
    enabled: !!slug,
  });
}

export function useChannelUsageSummary(filter: UsageFilter) {
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useQuery({
    queryKey: [slug, 'channel-usage-summary', filter],
    queryFn: () => queryChannelUsageSummary(filter),
    enabled: !!slug,
  });
}

export function useDailyUsage(filter: UsageFilter) {
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useQuery({
    queryKey: [slug, 'daily-usage', filter],
    queryFn: () => queryDailyUsage(filter),
    enabled: !!slug,
  });
}
