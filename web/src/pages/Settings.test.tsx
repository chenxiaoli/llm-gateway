import { describe, it, expect, beforeEach } from 'vitest';
import { renderWithProviders } from '../test/render';
import { screen, waitFor, fireEvent } from '@testing-library/react';
import Settings from './Settings';
import { setToken, clearToken } from '../api/client';

beforeEach(() => {
  clearToken();
  setToken('test-jwt-token');
});

describe('Settings page', () => {
  it('renders settings page with Settings title', async () => {
    renderWithProviders(<Settings />, { route: '/admin/settings' });

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'Settings' })).toBeInTheDocument();
    }, { timeout: 5000 });
  });

  it('renders tab navigation with all four tabs', async () => {
    renderWithProviders(<Settings />, { route: '/admin/settings' });

    await waitFor(() => {
      expect(screen.getByText('General')).toBeInTheDocument();
      expect(screen.getByText('Security & Audit')).toBeInTheDocument();
      expect(screen.getByText('System Info')).toBeInTheDocument();
      expect(screen.getByText('About')).toBeInTheDocument();
    }, { timeout: 5000 });
  });

  it('shows Allow Registration in General tab by default', async () => {
    renderWithProviders(<Settings />, { route: '/admin/settings' });

    await waitFor(() => {
      expect(screen.getByText('Allow Registration')).toBeInTheDocument();
    }, { timeout: 5000 });
  });

  it('renders 1 toggle in General tab (Allow Registration)', async () => {
    renderWithProviders(<Settings />, { route: '/admin/settings' });

    await waitFor(() => {
      expect(screen.getAllByRole('switch')).toHaveLength(1);
    }, { timeout: 5000 });
  });

  it('shows audit toggles when Security tab is clicked', async () => {
    renderWithProviders(<Settings />, { route: '/admin/settings' });

    await waitFor(() => {
      expect(screen.getByText('General')).toBeInTheDocument();
    }, { timeout: 5000 });

    fireEvent.click(screen.getByText('Security & Audit'));

    await waitFor(() => {
      expect(screen.getByText('Log Request Body')).toBeInTheDocument();
      expect(screen.getByText('Log Response Body')).toBeInTheDocument();
      expect(screen.getAllByRole('switch')).toHaveLength(2);
    }, { timeout: 5000 });
  });

  it('shows system info when System Info tab is clicked', async () => {
    renderWithProviders(<Settings />, { route: '/admin/settings' });

    await waitFor(() => {
      expect(screen.getByText('General')).toBeInTheDocument();
    }, { timeout: 5000 });

    fireEvent.click(screen.getByText('System Info'));

    await waitFor(() => {
      expect(screen.getByText(/config\.toml/i)).toBeInTheDocument();
      expect(screen.getByText('Server Bind Address')).toBeInTheDocument();
      expect(screen.getByText('Database Driver')).toBeInTheDocument();
    }, { timeout: 5000 });
  });

  it('shows about info when About tab is clicked', async () => {
    renderWithProviders(<Settings />, { route: '/admin/settings' });

    await waitFor(() => {
      expect(screen.getByText('General')).toBeInTheDocument();
    }, { timeout: 5000 });

    fireEvent.click(screen.getByText('About'));

    await waitFor(() => {
      expect(screen.getByText(/GitHub/i)).toBeInTheDocument();
      expect(screen.getByText(/v\d+\.\d+\.\d+/)).toBeInTheDocument();
    }, { timeout: 5000 });
  });

  it('does not render change password form (moved to dedicated page)', async () => {
    renderWithProviders(<Settings />, { route: '/admin/settings' });

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'Settings' })).toBeInTheDocument();
    }, { timeout: 5000 });

    expect(screen.queryByText('Current Password')).not.toBeInTheDocument();
  });
});
