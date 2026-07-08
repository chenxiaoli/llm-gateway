import { apiClient, orgPrefix } from './client';
import type { OrgSummary } from '../types';

/**
 * PATCH /api/v1/{org_slug} — update the current org's name and/or slug.
 * Admin+ gate on the backend; returns the updated OrgSummary (role echoed
 * back from the caller's membership, unaffected by the rename).
 */
export async function updateOrg(req: { name?: string; slug?: string }): Promise<OrgSummary> {
  const { data } = await apiClient.patch<OrgSummary>(orgPrefix(), req);
  return data;
}

/**
 * DELETE /api/v1/{org_slug} — hard-delete the current org.
 * Owner-only + password re-check on the backend. Cascades to all
 * org-scoped rows (keys, channels, usage, audit logs, members).
 * Returns 204 on success; caller must log out (auth state is now invalid).
 */
export async function deleteOrg(password: string): Promise<void> {
  await apiClient.delete(orgPrefix(), { data: { password } });
}
