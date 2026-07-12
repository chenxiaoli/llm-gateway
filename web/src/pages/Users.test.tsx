import { describe, it, expect } from 'vitest';
import { Toaster } from 'sonner';
import { renderWithProviders } from '../test/render';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import Users from './Users';

describe('Users page', () => {
  it('renders users table', async () => {
    renderWithProviders(<Users />, { route: '/admin/users' });

    await waitFor(() => {
      expect(screen.getByRole('table')).toBeInTheDocument();
    }, { timeout: 5000 });
  });

  it('shows Users title', async () => {
    renderWithProviders(<Users />, { route: '/admin/users' });

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'Users' })).toBeInTheDocument();
    }, { timeout: 5000 });
  });

  it('has columns: Username, Role, Group, Status, Created, Actions', async () => {
    renderWithProviders(<Users />, { route: '/admin/users' });

    await waitFor(() => {
      expect(screen.getByText('Username')).toBeInTheDocument();
      expect(screen.getByText('Role')).toBeInTheDocument();
      expect(screen.getByText('Group')).toBeInTheDocument();
      expect(screen.getByText('Status')).toBeInTheDocument();
      expect(screen.getByText('Created')).toBeInTheDocument();
      expect(screen.getByText('Actions')).toBeInTheDocument();
    }, { timeout: 5000 });
  });

  it('shows group selector in user drawer and updates on change', async () => {
    let patchBody: unknown = null;
    server.use(
      http.get('*/api/v1/test-org/admin/groups', () =>
        HttpResponse.json({
          items: [
            { id: 'g1', name: 'engineering', description: null, created_at: '', updated_at: '' },
            { id: 'g2', name: 'marketing', description: null, created_at: '', updated_at: '' },
          ],
          total: 2,
          page: 1,
          page_size: 20,
        }),
      ),
      http.get('*/api/v1/test-org/admin/users', () =>
        HttpResponse.json({
          items: [
            {
              id: 'u1',
              username: 'alice',
              role: 'user',
              enabled: true,
              group_id: null,
              group_name: null,
              balance: 100,
              threshold: 10,
              created_at: '2026-06-01T00:00:00Z',
              updated_at: '2026-06-01T00:00:00Z',
            },
          ],
          total: 1,
          page: 1,
          page_size: 20,
        }),
      ),
      http.get('*/api/v1/test-org/admin/users/u1/balance', () =>
        HttpResponse.json({
          account: { id: 'a1', user_id: 'u1', balance: 100, threshold: 10, created_at: '', updated_at: '' },
          transactions: { items: [], total: 0, page: 1, page_size: 10 },
        }),
      ),
      http.patch('*/api/v1/test-org/admin/users/u1', async ({ request }) => {
        patchBody = await request.json();
        return HttpResponse.json({
          id: 'u1', username: 'alice', role: 'user', enabled: true,
          group_id: 'g1', group_name: 'engineering',
          balance: 100, threshold: 10, created_at: '', updated_at: '',
        });
      }),
    );

    renderWithProviders(
      <>
        <Users />
        <Toaster />
      </>,
      { route: '/admin/users' },
    );

    await userEvent.click(await screen.findByText('alice'));
    // Drawer is open — find the group Select by its 'g1' option (the table also has a
    // "Group" column header, so don't use getByText('Group') to detect the drawer).
    const selects = screen.getAllByRole('combobox');
    const groupSelect = selects.find((s) =>
      Array.from(s.querySelectorAll('option')).some((o) => o.value === 'g1'),
    )!;
    expect(groupSelect).toBeDefined();
    await userEvent.selectOptions(groupSelect, 'g1');

    await waitFor(() => {
      expect(patchBody).toEqual({ group_id: 'g1' });
    });
  });
});
