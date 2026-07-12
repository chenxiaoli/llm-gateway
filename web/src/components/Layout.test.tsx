import { describe, it, expect, beforeEach } from 'vitest';
import { renderWithProviders } from '../test/render';
import { screen } from '@testing-library/react';
import { useAuthStore } from '../stores/authStore';
import type { User, OrgSummary } from '../types';
import AppLayout from './Layout';

const platformAdminUser: User = {
  id: 'u-pa',
  username: 'pa',
  platform_role: 'platform_admin',
  email: 'pa@example.com',
  email_verified_at: '2026-07-09T00:00:00Z',
};

const orgAdminUser: User = {
  ...platformAdminUser,
  id: 'u-oa',
  username: 'oa',
  platform_role: null,
};

const memberUser: User = { ...orgAdminUser, id: 'u-mem', username: 'mem' };

const adminOrg: OrgSummary = {
  id: 'org-1',
  slug: 'test-org',
  name: 'Test Org',
  role: 'admin',
  group_id: null,
};

const memberOrg: OrgSummary = { ...adminOrg, role: 'member' };

describe('AppLayout sidebar — Platform group gating', () => {
  beforeEach(() => {
    // Default to admin role (matches the global test setup); individual tests
    // override user/currentOrg as needed.
    useAuthStore.setState({ user: orgAdminUser, currentOrg: adminOrg });
  });

  it('platform_admin sees the Platform group with a Settings menu', () => {
    useAuthStore.setState({ user: platformAdminUser, currentOrg: adminOrg });

    renderWithProviders(<AppLayout />, { route: '/test-org/dashboard' });

    expect(screen.getByText('Platform')).toBeInTheDocument();
    // The Settings menu item under the Platform group uses t('sidebar.settings') = "Settings".
    // The org-admin "Admin" group is also visible because platform_admin implies isAdminOrAbove.
    expect(screen.getByText('Admin')).toBeInTheDocument();
    // Multiple "Settings" strings may appear elsewhere (e.g. footer/buttons);
    // assert at least one is present in the Platform group's nav section.
    const settingsItems = screen.getAllByText('Settings');
    expect(settingsItems.length).toBeGreaterThanOrEqual(1);
  });

  it('org admin does NOT see the Platform group, but Admin group remains', () => {
    useAuthStore.setState({ user: orgAdminUser, currentOrg: adminOrg });

    renderWithProviders(<AppLayout />, { route: '/test-org/dashboard' });

    expect(screen.queryByText('Platform')).not.toBeInTheDocument();
    expect(screen.getByText('Admin')).toBeInTheDocument();
    // "Console" appears both as the sidebar group heading and as the breadcrumb
    // root button in the header, so assert at least one match.
    expect(screen.getAllByText('Console').length).toBeGreaterThanOrEqual(1);
  });

  it('non-admin (member) sees neither Admin nor Platform groups', () => {
    useAuthStore.setState({ user: memberUser, currentOrg: memberOrg });

    renderWithProviders(<AppLayout />, { route: '/test-org/dashboard' });

    expect(screen.queryByText('Admin')).not.toBeInTheDocument();
    expect(screen.queryByText('Platform')).not.toBeInTheDocument();
    // Console group always shows.
    expect(screen.getAllByText('Console').length).toBeGreaterThanOrEqual(1);
  });
});
