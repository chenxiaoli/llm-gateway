import { describe, it, expect } from 'vitest';
import { Toaster } from 'sonner';
import { renderWithProviders } from '../test/render';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import Members from './Members';
import { useAuthStore } from '../stores/authStore';
import type { Member, OrgSummary } from '../types';

// Members page is rendered at /:slug/members — the slug in test setup is
// 'test-org' (see test/setup.ts). Render with that route so any route-aware
// hooks (useNavigate, useParams) work as in production.
const ROUTE = '/test-org/members';

// The default seeded user has role 'admin' in test/setup.ts. The Members page
// gates the Status toggle and Remove button behind canManage (admin+), so the
// admin role is what we want for most tests. Some tests want owner-level
// actions (e.g. role change for owners), but for the assertions here admin
// suffices.
const adminOrg: OrgSummary = {
  id: 'org-1',
  slug: 'test-org',
  name: 'Test Org',
  role: 'admin',
  group_id: null,
};

function seedAdmin() {
  useAuthStore.setState({ currentOrg: adminOrg });
}

const memberFixture: Member = {
  user_id: 'u-alice',
  username: 'alice',
  email: 'alice@example.com',
  role: 'member',
  group_id: 'g-engineering',
  group_name: 'engineering',
  enabled: true,
  balance: 50,
  threshold: 10,
  created_at: '2026-06-01T00:00:00Z',
};

describe('Members page', () => {
  it('renders the members table with all columns and the seeded row data', async () => {
    seedAdmin();
    server.use(
      http.get('*/api/v1/test-org/members', () => HttpResponse.json([memberFixture])),
    );

    renderWithProviders(<Members />, { route: ROUTE });

    // Column headers (Username, Role, Group, Status, Balance, Joined, Actions).
    await waitFor(() => {
      expect(screen.getByText('Username')).toBeInTheDocument();
    });
    expect(screen.getByText('Role')).toBeInTheDocument();
    expect(screen.getByText('Group')).toBeInTheDocument();
    expect(screen.getByText('Status')).toBeInTheDocument();
    expect(screen.getByText('Balance')).toBeInTheDocument();
    expect(screen.getByText('Joined')).toBeInTheDocument();
    expect(screen.getByText('Actions')).toBeInTheDocument();

    // Row data: username, group name, status badge, and formatted balance.
    expect(await screen.findByText('alice')).toBeInTheDocument();
    expect(screen.getByText('engineering')).toBeInTheDocument();
    expect(screen.getByText('Enabled')).toBeInTheDocument();
    // Balance is formatted as $50.00 (2 decimals, USD symbol default).
    expect(screen.getByText('$50.00')).toBeInTheDocument();
  });

  it('clicking the Detail button opens the member drawer', async () => {
    seedAdmin();
    server.use(
      http.get('*/api/v1/test-org/members', () => HttpResponse.json([memberFixture])),
      // Drawer fetches balance + groups on open.
      http.get('*/api/v1/test-org/admin/members/*/balance', () =>
        HttpResponse.json({
          account: {
            id: 'acct-1',
            user_id: 'u-alice',
            balance: 50,
            threshold: 10,
            created_at: '2026-06-01T00:00:00Z',
            updated_at: '2026-06-01T00:00:00Z',
          },
          transactions: { items: [], total: 0, page: 1, page_size: 10 },
        }),
      ),
      http.get('*/api/v1/test-org/admin/groups', () =>
        HttpResponse.json({ items: [], total: 0, page: 1, page_size: 20 }),
      ),
    );

    renderWithProviders(<Members />, { route: ROUTE });

    // Wait for the row to render.
    expect(await screen.findByText('alice')).toBeInTheDocument();

    // Click the Detail button for that row. Multiple rows could theoretically
    // exist, but with one seeded member there's exactly one Detail button.
    await userEvent.click(screen.getByRole('button', { name: 'Detail' }));

    // Drawer title is the users.drawer.title translation ("User Details").
    await waitFor(() => {
      expect(screen.getByText('User Details')).toBeInTheDocument();
    });
  });

  it('clicking the status toggle PATCHes {enabled: false} for an enabled member', async () => {
    seedAdmin();
    let patchBody: unknown = undefined;
    let patchUrl = '';
    server.use(
      http.get('*/api/v1/test-org/members', () => HttpResponse.json([memberFixture])),
      http.patch('*/api/v1/test-org/members/:userId', async ({ request, params }) => {
        patchBody = await request.json();
        patchUrl = params.userId as string;
        return HttpResponse.json({ ...memberFixture, enabled: false });
      }),
    );

    renderWithProviders(<Members />, { route: ROUTE });

    // Status badge shows "Enabled" for the seeded member.
    const statusBadge = await screen.findByText('Enabled');
    expect(statusBadge).toBeInTheDocument();

    // Click the status badge — the parent <button> issues the PATCH.
    await userEvent.click(statusBadge);

    await waitFor(() => {
      expect(patchBody).toEqual({ enabled: false });
      expect(patchUrl).toBe('u-alice');
    });
  });

  it('recharge form submission POSTs to /admin/members/:userId/recharge', async () => {
    seedAdmin();
    let rechargeBody: unknown = undefined;
    let rechargeUrl = '';
    server.use(
      http.get('*/api/v1/test-org/members', () => HttpResponse.json([memberFixture])),
      http.get('*/api/v1/test-org/admin/members/*/balance', () =>
        HttpResponse.json({
          account: {
            id: 'acct-1',
            user_id: 'u-alice',
            balance: 50,
            threshold: 10,
            created_at: '2026-06-01T00:00:00Z',
            updated_at: '2026-06-01T00:00:00Z',
          },
          transactions: { items: [], total: 0, page: 1, page_size: 10 },
        }),
      ),
      http.get('*/api/v1/test-org/admin/groups', () =>
        HttpResponse.json({ items: [], total: 0, page: 1, page_size: 20 }),
      ),
      http.post('*/api/v1/test-org/admin/members/:userId/recharge', async ({ request, params }) => {
        rechargeBody = await request.json();
        rechargeUrl = params.userId as string;
        return HttpResponse.json({
          id: 'acct-1',
          user_id: 'u-alice',
          balance: 100,
          threshold: 10,
          created_at: '2026-06-01T00:00:00Z',
          updated_at: new Date().toISOString(),
        });
      }),
    );

    renderWithProviders(
      <>
        <Members />
        <Toaster />
      </>,
      { route: ROUTE },
    );

    // Open the drawer.
    await userEvent.click(await screen.findByRole('button', { name: 'Detail' }));

    // Click Recharge inside the drawer.
    await userEvent.click(await screen.findByRole('button', { name: 'Recharge' }));

    // Fill the amount and submit.
    const amountInput = await screen.findByPlaceholderText('0.00');
    await userEvent.type(amountInput, '25');
    await userEvent.click(screen.getByRole('button', { name: 'Confirm Recharge' }));

    await waitFor(() => {
      expect(rechargeBody).toEqual({ amount: 25, description: 'Credit', type: 'credit' });
      expect(rechargeUrl).toBe('u-alice');
    });
  });

  it('clicking Deduct in the drawer POSTs to /recharge with type:debit', async () => {
    seedAdmin();
    let rechargeBody: unknown = undefined;
    server.use(
      http.get('*/api/v1/test-org/members', () => HttpResponse.json([memberFixture])),
      http.get('*/api/v1/test-org/admin/members/*/balance', () =>
        HttpResponse.json({
          account: {
            id: 'acct-1',
            user_id: 'u-alice',
            balance: 50,
            threshold: 10,
            created_at: '2026-06-01T00:00:00Z',
            updated_at: '2026-06-01T00:00:00Z',
          },
          transactions: { items: [], total: 0, page: 1, page_size: 10 },
        }),
      ),
      http.get('*/api/v1/test-org/admin/groups', () =>
        HttpResponse.json({ items: [], total: 0, page: 1, page_size: 20 }),
      ),
      http.post('*/api/v1/test-org/admin/members/:userId/recharge', async ({ request }) => {
        rechargeBody = await request.json();
        return HttpResponse.json({
          id: 'acct-1',
          user_id: 'u-alice',
          balance: 40,
          threshold: 10,
          created_at: '2026-06-01T00:00:00Z',
          updated_at: new Date().toISOString(),
        });
      }),
    );

    renderWithProviders(
      <>
        <Members />
        <Toaster />
      </>,
      { route: ROUTE },
    );

    // Open the drawer.
    await userEvent.click(await screen.findByRole('button', { name: 'Detail' }));

    // Click Deduct (the new button next to Recharge). Both routes through the
    // same modal — the only client-side difference is the `type` field.
    await userEvent.click(await screen.findByRole('button', { name: 'Deduct' }));

    const amountInput = await screen.findByPlaceholderText('0.00');
    await userEvent.type(amountInput, '10');
    await userEvent.click(screen.getByRole('button', { name: 'Confirm Deduction' }));

    await waitFor(() => {
      expect(rechargeBody).toEqual({ amount: 10, description: 'Debit', type: 'debit' });
    });
  });
});
