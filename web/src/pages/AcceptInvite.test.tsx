import { describe, it, expect, beforeEach, vi } from 'vitest';
import { renderWithProviders } from '../test/render';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import AcceptInvite from './AcceptInvite';
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

const authedUser: User = {
  id: 'user-1',
  username: 'alice',
  platform_role: null,
  email: 'alice@example.com',
  email_verified_at: '2026-01-01T00:00:00Z',
};

const invitingOrg: OrgSummary = {
  id: 'org-acme',
  slug: 'acme',
  name: 'Acme Inc.',
  role: 'member',
  group_id: null,
};

const previewBody = {
  org_name: 'Acme Inc.',
  org_slug: 'acme',
  role: 'member',
  inviter_username: 'inviter',
  recipient_email: 'alice@example.com',
  expires_at: '2026-12-31T00:00:00Z',
};

const authResponse = {
  token: 'fresh-jwt',
  refresh_token: 'fresh-refresh',
  user: authedUser,
  current_org: invitingOrg,
  orgs: [invitingOrg],
};

const meAfterAccept = {
  id: 'user-1',
  username: 'alice',
  platform_role: null,
  current_org: invitingOrg,
  orgs: [invitingOrg],
  allow_registration: true,
  impersonating: false,
};

function seedLoggedOut() {
  useAuthStore.setState({
    user: null,
    currentOrg: null,
    orgs: [],
    impersonating: false,
    pendingInviteToken: null,
  });
}

function seedLoggedInNoMember() {
  useAuthStore.setState({
    user: authedUser,
    currentOrg: null,
    orgs: [],
    impersonating: false,
    pendingInviteToken: null,
  });
}

function seedLoggedInAlreadyMember() {
  useAuthStore.setState({
    user: authedUser,
    currentOrg: invitingOrg,
    orgs: [invitingOrg],
    impersonating: false,
    pendingInviteToken: null,
  });
}

function seedLoggedInMismatchedEmail() {
  useAuthStore.setState({
    user: { ...authedUser, email: 'bob@example.com' },
    currentOrg: null,
    orgs: [],
    impersonating: false,
    pendingInviteToken: null,
  });
}

function seedLoggedInUnverifiedEmail() {
  // Same email as the preview's recipient, but unverified.
  useAuthStore.setState({
    user: { ...authedUser, email_verified_at: null },
    currentOrg: null,
    orgs: [],
    impersonating: false,
    pendingInviteToken: null,
  });
}

beforeEach(() => {
  mockNavigate.mockClear();
  mockToastError.mockClear();
  clearToken();
  clearRefreshToken();
});

describe('AcceptInvite page', () => {
  it('logged out: shows org metadata + signup/login buttons', async () => {
    seedLoggedOut();
    server.use(
      http.get('*/api/v1/invitations/preview', () => HttpResponse.json(previewBody)),
    );

    renderWithProviders(<AcceptInvite />, { route: '/accept-invite?token=abc123' });

    await waitFor(() => {
      expect(screen.getByText(/Join Acme Inc\./)).toBeInTheDocument();
    });
    expect(screen.getByText('Invite from inviter')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Sign up to accept' })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Log in' })).toBeInTheDocument();
  });

  it('logged out: 410 preview error shows "gone" UI', async () => {
    seedLoggedOut();
    server.use(
      http.get('*/api/v1/invitations/preview', () =>
        HttpResponse.json(
          { error: { message: 'invitation expired' } },
          { status: 410 },
        ),
      ),
    );

    renderWithProviders(<AcceptInvite />, { route: '/accept-invite?token=expired' });

    await waitFor(() => {
      expect(screen.getByText('Invitation no longer valid')).toBeInTheDocument();
    });
    expect(screen.queryByRole('button', { name: 'Sign up to accept' })).not.toBeInTheDocument();
  });

  it('logged in: shows Accept/Decline buttons (not already a member)', async () => {
    seedLoggedInNoMember();
    server.use(
      http.get('*/api/v1/invitations/preview', () => HttpResponse.json(previewBody)),
    );

    renderWithProviders(<AcceptInvite />, { route: '/accept-invite?token=abc123' });

    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Accept' })).toBeInTheDocument();
    });
    expect(screen.getByRole('link', { name: 'Decline' })).toBeInTheDocument();
    expect(screen.queryByText(/already a member/i)).not.toBeInTheDocument();
  });

  it('logged in: accept click POSTs /invitations/accept and navigates to org dashboard', async () => {
    seedLoggedInNoMember();
    server.use(
      http.get('*/api/v1/invitations/preview', () => HttpResponse.json(previewBody)),
      http.post('*/api/v1/invitations/accept', () => HttpResponse.json(authResponse)),
      http.get('*/api/v1/auth/me', () => HttpResponse.json(meAfterAccept)),
    );

    renderWithProviders(<AcceptInvite />, { route: '/accept-invite?token=abc123' });

    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Accept' })).toBeInTheDocument();
    });
    await userEvent.click(screen.getByRole('button', { name: 'Accept' }));

    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith('/acme/dashboard', { replace: true });
    });
  });

  it('logged in: already a member of inviting org shows informational message + Go to org link', async () => {
    seedLoggedInAlreadyMember();
    server.use(
      http.get('*/api/v1/invitations/preview', () => HttpResponse.json(previewBody)),
    );

    renderWithProviders(<AcceptInvite />, { route: '/accept-invite?token=abc123' });

    await waitFor(() => {
      expect(screen.getByText(/already a member of Acme Inc\./)).toBeInTheDocument();
    });
    expect(screen.getByRole('link', { name: 'Go to Acme Inc.' })).toHaveAttribute(
      'href',
      '/acme/dashboard',
    );
    // Accept/Decline buttons should NOT be present for already-members.
    expect(screen.queryByRole('button', { name: 'Accept' })).not.toBeInTheDocument();
  });

  it('logged in: mismatched email shows mismatch notice and no Accept button', async () => {
    seedLoggedInMismatchedEmail();
    server.use(
      http.get('*/api/v1/invitations/preview', () => HttpResponse.json(previewBody)),
    );

    renderWithProviders(<AcceptInvite />, { route: '/accept-invite?token=abc123' });

    // Mismatch notice mentions the invitation's recipient email.
    await waitFor(() => {
      expect(screen.getByText(/This invitation was sent to alice@example\.com/)).toBeInTheDocument();
    });
    // No Accept button — they can't take this invitation with this account.
    expect(screen.queryByRole('button', { name: 'Accept' })).not.toBeInTheDocument();
  });

  it('logged in: unverified email shows verify-first notice and no Accept button', async () => {
    seedLoggedInUnverifiedEmail();
    server.use(
      http.get('*/api/v1/invitations/preview', () => HttpResponse.json(previewBody)),
    );

    renderWithProviders(<AcceptInvite />, { route: '/accept-invite?token=abc123' });

    await waitFor(() => {
      expect(screen.getByText('Verify your email first, then come back to accept.')).toBeInTheDocument();
    });
    expect(screen.queryByRole('button', { name: 'Accept' })).not.toBeInTheDocument();
  });
});
