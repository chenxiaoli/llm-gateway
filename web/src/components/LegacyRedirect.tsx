import { Navigate, useLocation } from 'react-router-dom';
import { useAuthStore } from '../stores/authStore';

/**
 * Redirect /console/* and /admin/* (pre-Phase-2 paths) to their /:orgSlug/*
 * equivalents. If no currentOrg, bounce to /login.
 *
 * /console/keys → /${currentOrg.slug}/keys
 * /admin/channels → /${currentOrg.slug}/admin/channels
 */
export function LegacyRedirect() {
  const location = useLocation();
  const currentOrg = useAuthStore((s) => s.currentOrg);
  if (!currentOrg) return <Navigate to="/login" replace />;

  const isAdmin = location.pathname.startsWith('/admin');
  const tail = location.pathname.replace(/^\/(console|admin)/, '');
  const newPath = `/${currentOrg.slug}${isAdmin ? '/admin' : ''}${tail}`;
  return <Navigate to={newPath} replace />;
}
