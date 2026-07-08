import { useMemo } from 'react';
import type { CatalogScope } from '../components/CatalogFilter';

export function useCatalogFilter<T extends { owner_org_id: string | null }>(
  data: T[] | undefined,
  scope: CatalogScope,
): T[] | undefined {
  return useMemo(() => {
    if (!data) return undefined;
    return data.filter((item) => {
      if (scope === 'all') return true;
      if (scope === 'platform') return item.owner_org_id === null;
      return item.owner_org_id !== null;
    });
  }, [data, scope]);
}
