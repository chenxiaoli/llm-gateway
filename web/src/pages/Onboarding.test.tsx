import { describe, it, expect, beforeEach, vi } from 'vitest';
import { renderWithProviders } from '../test/render';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import Onboarding from './Onboarding';
import { useAuthStore } from '../stores/authStore';
import { clearToken, clearRefreshToken } from '../api/client';
import type { User, OrgSummary } from '../types';

const { mockNavigate, mockToastError } = vi.hoisted(() => ({
  mockNavigate: vi.fn(),
  mockToastError: vi.fn(),
}));

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom');
  return { ...actual, useNavigate: () => mockNavigate };
});
vi.mock('sonner', () => ({ toast: { error: mockToastError, success: vi.fn() } }));

const limboUser: User = {
  id: 'user-limbo',
  username: 'limbo',
  platform_role: null,
};

function seedLimboUser() {
  useAuthStore.setState({
    user: limboUser,
    currentOrg: null,
    orgs: [],
    impersonating: false,
  });
}

const newOrg: OrgSummary = {
  id: 'org-new',
  slug: 'acme',
  name: 'Acme',
  role: 'owner',
  group_id: null,
};

const meAfterOnboarding = {
  id: 'user-limbo',
  username: 'limbo',
  platform_role: null,
  current_org: newOrg,
  orgs: [newOrg],
  allow_registration: true,
  impersonating: false,
};

const authResponse = {
  token: 'fresh-jwt',
  refresh_token: 'fresh-refresh',
  user: limboUser,
  current_org: newOrg,
  orgs: [newOrg],
};

beforeEach(() => {
  mockNavigate.mockClear();
  mockToastError.mockClear();
  // Clear any tokens that applyAuthResponse persisted in a prior test —
  // otherwise OnboardingGate treats the stale token as "loading" and the
  // wizard never mounts.
  clearToken();
  clearRefreshToken();
  seedLimboUser();
});

describe('Onboarding wizard', () => {
  it('shows two branch cards by default', () => {
    renderWithProviders(<Onboarding />, { route: '/onboarding' });

    expect(screen.getByRole('heading', { name: /create an org/i })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: /have an invite/i })).toBeInTheDocument();
  });

  it('create branch: shows slug-collision error on 409 from /orgs', async () => {
    server.use(
      http.post('*/api/v1/orgs', () =>
        HttpResponse.json(
          { error: { message: 'org slug already taken' } },
          { status: 409 },
        ),
      ),
    );

    renderWithProviders(<Onboarding />, { route: '/onboarding' });

    await userEvent.type(
      screen.getByPlaceholderText('e.g., Acme Inc.'),
      'Acme',
    );
    // slug auto-fills from name; ensure it has a value then submit
    const slugInput = screen.getByPlaceholderText('e.g., acme');
    await userEvent.clear(slugInput);
    await userEvent.type(slugInput, 'acme');

    await userEvent.click(screen.getByRole('button', { name: 'Create' }));

    await waitFor(() => {
      expect(screen.getByText('That slug is taken, try another')).toBeInTheDocument();
    });
    expect(mockNavigate).not.toHaveBeenCalled();
  });

  it('create branch: success applies auth response and redirects to dashboard', async () => {
    server.use(
      http.post('*/api/v1/orgs', () => HttpResponse.json(authResponse)),
      http.get('*/api/v1/auth/me', () => HttpResponse.json(meAfterOnboarding)),
    );

    renderWithProviders(<Onboarding />, { route: '/onboarding' });

    await userEvent.type(
      screen.getByPlaceholderText('e.g., Acme Inc.'),
      'Acme',
    );
    const slugInput = screen.getByPlaceholderText('e.g., acme');
    await userEvent.clear(slugInput);
    await userEvent.type(slugInput, 'acme');

    await userEvent.click(screen.getByRole('button', { name: 'Create' }));

    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith('/acme/dashboard', { replace: true });
    });
  });

  it('join branch: success redirects to inviting org dashboard', async () => {
    server.use(
      http.post('*/api/v1/invitations/accept', () => HttpResponse.json(authResponse)),
      http.get('*/api/v1/auth/me', () => HttpResponse.json(meAfterOnboarding)),
    );

    renderWithProviders(<Onboarding />, { route: '/onboarding' });

    await userEvent.type(
      screen.getByPlaceholderText('Paste your invitation link or token'),
      'invitetoken123',
    );
    await userEvent.click(screen.getByRole('button', { name: 'Join' }));

    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith('/acme/dashboard', { replace: true });
    });
  });

  it('join branch: 410 shows invalid-token error', async () => {
    server.use(
      http.post('*/api/v1/invitations/accept', () =>
        HttpResponse.json(
          { error: { message: 'invitation expired' } },
          { status: 410 },
        ),
      ),
    );

    renderWithProviders(<Onboarding />, { route: '/onboarding' });

    await userEvent.type(
      screen.getByPlaceholderText('Paste your invitation link or token'),
      'expiredtoken',
    );
    await userEvent.click(screen.getByRole('button', { name: 'Join' }));

    await waitFor(() => {
      expect(screen.getByText('This invitation is no longer valid')).toBeInTheDocument();
    });
    expect(mockNavigate).not.toHaveBeenCalled();
  });

  it('join branch: accepts a full accept-invite URL and extracts the token', async () => {
    let receivedToken: string | undefined;
    server.use(
      http.post('*/api/v1/invitations/accept', async ({ request }) => {
        const body = (await request.json()) as { token: string };
        receivedToken = body.token;
        return HttpResponse.json(authResponse);
      }),
      http.get('*/api/v1/auth/me', () => HttpResponse.json(meAfterOnboarding)),
    );

    renderWithProviders(<Onboarding />, { route: '/onboarding' });

    await userEvent.type(
      screen.getByPlaceholderText('Paste your invitation link or token'),
      'https://app.example.com/accept-invite?token=urltoken456',
    );
    await userEvent.click(screen.getByRole('button', { name: 'Join' }));

    await waitFor(() => {
      expect(receivedToken).toBe('urltoken456');
    });
    expect(mockNavigate).toHaveBeenCalledWith('/acme/dashboard', { replace: true });
  });
});
