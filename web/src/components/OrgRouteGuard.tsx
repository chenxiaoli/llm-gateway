import { useEffect } from 'react';
import { useParams, Navigate, Outlet } from 'react-router-dom';
import { useAuthStore } from '../stores/authStore';

/**
 * Guards `/:orgSlug/*` routes. Three responsibilities:
 *
 * 1. If the slug isn't in `user.orgs` → redirect to current org's dashboard
 *    (or /login if no currentOrg).
 * 2. If the slug differs from `currentOrg.slug` → call `setCurrentOrg` first
 *    (persists to backend, rotates token, clears cache). Render happens on
 *    next render cycle after store update.
 * 3. Otherwise render <Outlet />.
 */
export function OrgRouteGuard() {
  const { orgSlug } = useParams<{ orgSlug: string }>();
  const currentOrg = useAuthStore((s) => s.currentOrg);
  const orgs = useAuthStore((s) => s.orgs);
  const setCurrentOrg = useAuthStore((s) => s.setCurrentOrg);

  const matched = orgs.find((o) => o.slug === orgSlug);

  useEffect(() => {
    if (matched && currentOrg?.slug !== matched.slug) {
      void setCurrentOrg(matched);
    }
  }, [matched, currentOrg, setCurrentOrg]);

  if (!matched) {
    return <Navigate to={currentOrg ? `/${currentOrg.slug}/dashboard` : '/login'} replace />;
  }

  return <Outlet />;
}
