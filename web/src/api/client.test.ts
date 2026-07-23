import { describe, it, expect, beforeEach } from 'vitest';
import { orgPrefix } from './client';
import { useAuthStore } from '../stores/authStore';
import type { OrgSummary } from '../types';

describe('orgPrefix', () => {
  beforeEach(() => {
    useAuthStore.setState({
      user: null,
      currentOrg: null,
      orgs: [],
      isLoading: false,
    });
  });

  it('throws when no current org is set', () => {
    expect(() => orgPrefix()).toThrow(/no current org/i);
  });

  it('returns the slug-relative path when currentOrg is set', () => {
    const org: OrgSummary = {
      id: 'org-1',
      slug: 'acme',
      name: 'Acme',
      role: 'admin',
      group_id: null,
    };
    useAuthStore.setState({ currentOrg: org });
    // Returns `/acme`, NOT `/api/v1/acme` — `apiClient.baseURL` already
    // carries `/api/v1`. Returning `/api/v1/${slug}` here caused every
    // org-scoped request to double the prefix (`/api/v1/api/v1/<slug>/...`).
    expect(orgPrefix()).toBe('/acme');
  });

  it('returns the slug-relative path for slug with hyphen', () => {
    const org: OrgSummary = {
      id: 'org-2',
      slug: 'my-company',
      name: 'My Company',
      role: 'owner',
      group_id: null,
    };
    useAuthStore.setState({ currentOrg: org });
    expect(orgPrefix()).toBe('/my-company');
  });
});
