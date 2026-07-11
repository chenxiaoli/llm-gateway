import { useQuery } from '@tanstack/react-query';
import { useAuthStore } from '../stores/authStore';
import { getBudgetStatus } from '../api/orgs';

/**
 * Phase 7: subscribe to the org's current-month MTD spend + month bucket.
 * Pairs with `useGetOrgDefaults` (which returns the budget cap) to render
 * the Budget status card on the OrgSettings page.
 */
export function useGetBudgetStatus() {
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useQuery({
    queryKey: [slug, 'budgetStatus'],
    queryFn: () => getBudgetStatus(),
    enabled: !!slug,
  });
}
