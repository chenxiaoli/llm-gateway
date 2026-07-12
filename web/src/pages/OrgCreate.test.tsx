import { describe, it, expect, beforeEach, vi } from 'vitest';
import { renderWithProviders } from '../test/render';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { useAuthStore } from '../stores/authStore';
import { clearToken, clearRefreshToken } from '../api/client';
import type { User, OrgSummary } from '../types';
import OrgCreate from './OrgCreate';

// Mock useNavigate so we can assert where the page would take the user
// after a successful create — without that assertion, success looks the
// same as failure (both just stop rendering the form).
const { mockNavigate } = vi.hoisted(() => ({ mockNavigate: vi.fn() }));
vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom');
  return { ...actual, useNavigate: () => mockNavigate };
});

const existingUser: User = {
  id: 'user-1',
  username: 'alice',
  platform_role: null,
  email: null,
  email_verified_at: '2026-01-01T00:00:00Z',
};

const existingOrg: OrgSummary = {
  id: 'org-old',
  slug: 'oldco',
  name: 'Old Co',
  role: 'owner',
  group_id: null,
};

const newOrg: OrgSummary = {
  id: 'org-new',
  slug: 'acme',
  name: 'Acme',
  role: 'owner',
  group_id: null,
};

const authResponse = {
  token: 'fresh-jwt',
  refresh_token: 'fresh-refresh',
  user: existingUser,
  current_org: newOrg,
  orgs: [existingOrg, newOrg],
};

// /auth/me response after switching to the new org — the auth store
// refetches via applyAuthResponse and this is what it gets back.
const meAfterCreate = {
  ...existingUser,
  current_org: newOrg,
  orgs: [existingOrg, newOrg],
  allow_registration: true,
  impersonating: false,
};

function seedLoggedInUser() {
  useAuthStore.setState({
    user: existingUser,
    currentOrg: existingOrg,
    orgs: [existingOrg],
    impersonating: false,
  });
}

beforeEach(() => {
  mockNavigate.mockClear();
  // Tokens from a prior test would make the gate enter "loading" state
  // waiting for /auth/me; clear so the gate settles immediately.
  clearToken();
  clearRefreshToken();
  seedLoggedInUser();
});

describe('OrgCreate page', () => {
  it('renders page title and the create form for a logged-in user', () => {
    renderWithProviders(<OrgCreate />, { route: '/orgs/new' });

    // Page-level copy from orgCreate.* (NOT onboarding.*).
    expect(screen.getByRole('heading', { name: /create a new org/i })).toBeInTheDocument();
    expect(screen.getByText(/you'll switch to the new org/i)).toBeInTheDocument();
    // Card-internal copy from onboarding.create.* (reused unchanged).
    expect(screen.getByPlaceholderText('e.g., Acme Inc.')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('e.g., acme')).toBeInTheDocument();
  });

  it('on submit, applies auth response and redirects to new org dashboard', async () => {
    server.use(
      http.post('*/api/v1/orgs', () => HttpResponse.json(authResponse)),
      http.get('*/api/v1/auth/me', () => HttpResponse.json(meAfterCreate)),
    );

    renderWithProviders(<OrgCreate />, { route: '/orgs/new' });

    await userEvent.type(screen.getByPlaceholderText('e.g., Acme Inc.'), 'Acme');
    const slugInput = screen.getByPlaceholderText('e.g., acme');
    await userEvent.clear(slugInput);
    await userEvent.type(slugInput, 'acme');

    await userEvent.click(screen.getByRole('button', { name: 'Create' }));

    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith('/acme/dashboard', { replace: true });
    });
    // Auth store now reflects the new current org.
    expect(useAuthStore.getState().currentOrg?.slug).toBe('acme');
  });
});
