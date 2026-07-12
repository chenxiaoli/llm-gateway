import { describe, it, expect, beforeEach, vi } from 'vitest';
import { renderWithProviders } from '../test/render';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { setToken } from '../api/client';
import { useAuthStore } from '../stores/authStore';
import PlatformUsers from './PlatformUsers';

const adminUser = {
  id: 'u-pa', username: 'pa', platform_role: 'platform_admin' as const,
  email: 'pa@x.com', email_verified_at: '2026-07-12T00:00:00Z',
};

beforeEach(() => {
  setToken('test-token');
  useAuthStore.setState({ user: adminUser });
});

describe('PlatformUsers page', () => {
  it('renders current platform admins from the API', async () => {
    server.use(
      http.get('/api/v1/admin/platform-users', () =>
        HttpResponse.json({
          admins: [
            { id: 'u-pa', username: 'pa', email: 'pa@x.com', platform_role: 'platform_admin' },
            { id: 'u-2', username: 'second', email: 's@x.com', platform_role: 'platform_admin' },
          ],
        }),
      ),
    );
    renderWithProviders(<PlatformUsers />);
    await waitFor(() => {
      expect(screen.getByText('pa')).toBeInTheDocument();
      expect(screen.getByText('second')).toBeInTheDocument();
    });
  });

  it('hides the revoke button when only one admin (self)', async () => {
    server.use(
      http.get('/api/v1/admin/platform-users', () =>
        HttpResponse.json({
          admins: [
            { id: 'u-pa', username: 'pa', email: 'pa@x.com', platform_role: 'platform_admin' },
          ],
        }),
      ),
    );
    renderWithProviders(<PlatformUsers />);
    await waitFor(() => expect(screen.getByText('pa')).toBeInTheDocument());
    expect(screen.queryByRole('button', { name: /revoke/i })).not.toBeInTheDocument();
  });

  it('shows revoke button when multiple admins exist', async () => {
    server.use(
      http.get('/api/v1/admin/platform-users', () =>
        HttpResponse.json({
          admins: [
            { id: 'u-pa', username: 'pa', email: 'pa@x.com', platform_role: 'platform_admin' },
            { id: 'u-2', username: 'second', email: 's@x.com', platform_role: 'platform_admin' },
          ],
        }),
      ),
    );
    renderWithProviders(<PlatformUsers />);
    await waitFor(() => expect(screen.getByText('pa')).toBeInTheDocument());
    const revokeButtons = screen.getAllByRole('button', { name: /revoke/i });
    expect(revokeButtons.length).toBeGreaterThanOrEqual(1);
  });

  it('PATCHes platform_role when revoke is clicked', async () => {
    let patched: { url: string; body: any } | null = null;
    server.use(
      http.get('/api/v1/admin/platform-users', () =>
        HttpResponse.json({
          admins: [
            { id: 'u-pa', username: 'pa', email: 'pa@x.com', platform_role: 'platform_admin' },
            { id: 'u-2', username: 'second', email: 's@x.com', platform_role: 'platform_admin' },
          ],
        }),
      ),
      http.patch('/api/v1/admin/users/:userId/platform-role', async ({ request }) => {
        patched = { url: request.url, body: await request.json() };
        return HttpResponse.json({ id: 'u-2', platform_role: null });
      }),
    );
    renderWithProviders(<PlatformUsers />);
    await waitFor(() => expect(screen.getByText('second')).toBeInTheDocument());
    const user = userEvent.setup();
    // Stub window.confirm so the dialog auto-accepts
    vi.spyOn(window, 'confirm').mockReturnValue(true);
    await user.click(screen.getAllByRole('button', { name: /revoke/i })[0]);
    await waitFor(() => {
      expect(patched).not.toBeNull();
      expect(patched!.body).toEqual({ platform_role: null });
    });
  });
});
