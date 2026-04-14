import { describe, it, expect, beforeEach } from 'vitest';
import { renderWithProviders } from '../test/render';
import { screen, waitFor } from '@testing-library/react';
import Settings from './Settings';
import { setToken, clearToken } from '../api/client';

beforeEach(() => {
  clearToken();
  setToken('test-jwt-token');
});

describe('Settings page', () => {
  it('renders settings page with Settings title', async () => {
    renderWithProviders(<Settings />, { route: '/console/settings' });

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'Settings' })).toBeInTheDocument();
    }, { timeout: 5000 });
  });

  it('shows Allow Registration label', async () => {
    renderWithProviders(<Settings />, { route: '/console/settings' });

    await waitFor(() => {
      expect(screen.getByText('Allow Registration')).toBeInTheDocument();
    }, { timeout: 5000 });
  });

  it('renders settings toggles', async () => {
    renderWithProviders(<Settings />, { route: '/console/settings' });

    await waitFor(() => {
      expect(screen.getAllByRole('switch')).toHaveLength(3);
    }, { timeout: 5000 });
  });

  it('renders change password form', async () => {
    renderWithProviders(<Settings />, { route: '/console/settings' });

    await waitFor(() => {
      expect(screen.getByText('Current Password')).toBeInTheDocument();
    }, { timeout: 5000 });

    expect(screen.getByText('New Password')).toBeInTheDocument();
    expect(screen.getByText('Confirm New Password')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Change Password' })).toBeInTheDocument();
  });
});
