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

  it('returns the prefixed path when currentOrg is set', () => {
    const org: OrgSummary = {
      id: 'org-1',
      slug: 'acme',
      name: 'Acme',
      role: 'admin',
      group_id: null,
    };
    useAuthStore.setState({ currentOrg: org });
    expect(orgPrefix()).toBe('/api/v1/acme');
  });

  it('returns the prefixed path for slug with hyphen', () => {
    const org: OrgSummary = {
      id: 'org-2',
      slug: 'my-company',
      name: 'My Company',
      role: 'owner',
      group_id: null,
    };
    useAuthStore.setState({ currentOrg: org });
    expect(orgPrefix()).toBe('/api/v1/my-company');
  });
});
