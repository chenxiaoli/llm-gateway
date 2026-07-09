import { describe, it, expect, beforeEach } from 'vitest';
import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithProviders } from '../test/render';
import { useAuthStore } from '../stores/authStore';
import type { User } from '../types';
import { EmailBanner } from './EmailBanner';

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
    expect(screen.getByText('Add an email to receive invitations and reset your password.')).toBeInTheDocument();
  });

  it('is hidden when user.email !== null', () => {
    useAuthStore.setState({ user: userWithEmail });
    renderWithProviders(<EmailBanner />);
    expect(screen.queryByText('Add an email to receive invitations and reset your password.')).not.toBeInTheDocument();
  });

  it('is hidden when emailBannerDismissed === true', () => {
    useAuthStore.setState({ emailBannerDismissed: true });
    renderWithProviders(<EmailBanner />);
    expect(screen.queryByText('Add an email to receive invitations and reset your password.')).not.toBeInTheDocument();
  });

  it('opens the AddEmailModal when "Add email" is clicked', async () => {
    renderWithProviders(<EmailBanner />);
    await userEvent.click(screen.getByRole('button', { name: 'Add email' }));
    expect(screen.getByText('Add your email')).toBeInTheDocument();
  });

  it('hides the banner when "Dismiss" is clicked', async () => {
    renderWithProviders(<EmailBanner />);
    await userEvent.click(screen.getByRole('button', { name: 'Dismiss' }));
    expect(screen.queryByText('Add an email to receive invitations and reset your password.')).not.toBeInTheDocument();
  });
});
