import { describe, it, expect, beforeEach } from 'vitest';
import { renderWithProviders } from '../test/render';
import { screen } from '@testing-library/react';
import { useAuthStore } from '../stores/authStore';
import type { User, OrgSummary } from '../types';
import PlatformLayout from './PlatformLayout';

const platformAdmin: User = {
  id: 'u-pa', username: 'pa', platform_role: 'platform_admin',
  email: 'pa@x.com', email_verified_at: '2026-07-12T00:00:00Z',
};
const org: OrgSummary = {
  id: 'org-1', slug: 'test-org', name: 'Test Org', role: 'admin', group_id: null,
};

describe('PlatformLayout', () => {
  beforeEach(() => {
    useAuthStore.setState({ user: platformAdmin, currentOrg: org });
  });

  it('renders Platform sidebar with Settings and Platform Users links', () => {
    renderWithProviders(<PlatformLayout />, { route: '/admin/settings' });
    expect(screen.getByText('Platform')).toBeInTheDocument();
    // The links are rendered as buttons inside the sidebar.
    const settingsLinks = screen.getAllByText('Settings');
    expect(settingsLinks.length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText('Platform Users')).toBeInTheDocument();
  });

  it('shows back-to-org link when currentOrg is set', () => {
    renderWithProviders(<PlatformLayout />, { route: '/admin/settings' });
    expect(screen.getByText(/back to/i)).toBeInTheDocument();
    expect(screen.getByText(/Test Org/)).toBeInTheDocument();
  });

  it('hides back-to-org link when currentOrg is null', () => {
    useAuthStore.setState({ currentOrg: null });
    renderWithProviders(<PlatformLayout />, { route: '/admin/settings' });
    expect(screen.queryByText(/back to/i)).not.toBeInTheDocument();
  });
});