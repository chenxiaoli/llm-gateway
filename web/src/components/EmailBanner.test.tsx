import { describe, it, expect, vi, beforeEach } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { renderWithProviders } from '../test/render';
import { useAuthStore } from '../stores/authStore';
import type { User } from '../types';
import { EmailBanner } from './EmailBanner';

const { mockToastSuccess } = vi.hoisted(() => ({
  mockToastSuccess: vi.fn(),
}));

vi.mock('sonner', () => ({
  toast: { success: mockToastSuccess, error: vi.fn() },
}));

const BANNER_TEXT = 'Add an email to receive invitations and reset your password.';

const userWithoutEmail: User = {
  id: 'u1',
  username: 'alice',
  platform_role: null,
  email: null,
  email_verified_at: null,
};

const userWithEmail: User = {
  id: 'u1',
  username: 'alice',
  platform_role: null,
  email: 'alice@example.com',
  email_verified_at: null,
};

describe('EmailBanner', () => {
  beforeEach(() => {
    useAuthStore.setState({
      user: userWithoutEmail,
      emailBannerDismissed: false,
    });
  });

  it('renders when user.email === null', () => {
    renderWithProviders(<EmailBanner />);
    expect(screen.getByText(BANNER_TEXT)).toBeInTheDocument();
  });

  it('is hidden when user.email !== null', () => {
    useAuthStore.setState({ user: userWithEmail });
    renderWithProviders(<EmailBanner />);
    expect(screen.queryByText(BANNER_TEXT)).not.toBeInTheDocument();
  });

  it('is hidden when emailBannerDismissed === true', () => {
    useAuthStore.setState({ emailBannerDismissed: true });
    renderWithProviders(<EmailBanner />);
    expect(screen.queryByText(BANNER_TEXT)).not.toBeInTheDocument();
  });

  it('opens the AddEmailModal when "Add email" is clicked', async () => {
    renderWithProviders(<EmailBanner />);
    await userEvent.click(screen.getByRole('button', { name: 'Add email' }));
    expect(screen.getByText('Add your email')).toBeInTheDocument();
  });

  it('hides the banner when "Dismiss" is clicked', async () => {
    renderWithProviders(<EmailBanner />);
    await userEvent.click(screen.getByRole('button', { name: 'Dismiss' }));
    expect(screen.queryByText(BANNER_TEXT)).not.toBeInTheDocument();
  });

  // Integration: the banner must disappear after a successful email add. This
  // locks in the reactivity contract — the AddEmailModal writes via setUser,
  // EmailBanner's selector must observe the new user.email and unmount.
  it('disappears after the modal successfully sets an email', async () => {
    server.use(
      http.post('*/api/v1/auth/me/email', () =>
        HttpResponse.json({
          id: 'u1',
          username: 'alice',
          platform_role: null,
          current_org: null,
          orgs: [],
          allow_registration: true,
          impersonating: false,
          email: 'alice@example.com',
          email_verified_at: null,
          requires_email_verification: false,
        }),
      ),
    );

    renderWithProviders(<EmailBanner />);
    expect(screen.getByText(BANNER_TEXT)).toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: 'Add email' }));
    await userEvent.type(screen.getByLabelText('Email'), 'alice@example.com');
    await userEvent.click(screen.getByRole('button', { name: 'Send verification' }));

    await waitFor(() => {
      expect(screen.queryByText(BANNER_TEXT)).not.toBeInTheDocument();
    });
    expect(useAuthStore.getState().user?.email).toBe('alice@example.com');
  });
});
