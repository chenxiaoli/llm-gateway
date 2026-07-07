import type { User, OrgSummary } from '../types';

/**
 * The single canonical "is this user an admin" predicate for the frontend.
 *
 * Phase 1: admin routes/sidebar are gated by org role (owner/admin of the
 * current org) OR by platform_admin (rare, platform-wide). The backend
 * enforces actual permissions via membership — this is the UI mirror.
 *
 * Update this function when the gating rule changes; do not re-derive the
 * predicate at call sites.
 */
export function isAdminOrAbove(
  user: Pick<User, 'platform_role'> | null | undefined,
  currentOrg: Pick<OrgSummary, 'role'> | null | undefined,
): boolean {
  return (
    currentOrg?.role === 'owner' ||
    currentOrg?.role === 'admin' ||
    user?.platform_role === 'platform_admin'
  );
}
