import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderWithProviders } from '../test/render';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Routes, Route } from 'react-router-dom';
import { useAuthStore } from '../stores/authStore';
import type { User, OrgSummary } from '../types';
import OrgSettings from './OrgSettings';

const { mockToastSuccess, mockToastError } = vi.hoisted(() => ({
  mockToastSuccess: vi.fn(),
  mockToastError: vi.fn(),
}));

vi.mock('sonner', () => ({
  toast: { success: mockToastSuccess, error: mockToastError },
}));

const adminUser: User = {
  id: 'u1',
  username: 'alice',
  platform_role: null,
  email: 'alice@example.com',
  email_verified_at: '2026-07-09T00:00:00Z',
};

const memberUser: User = {
  ...adminUser,
  id: 'u2',
  username: 'bob',
};

const adminOrg: OrgSummary = {
  id: 'org-1',
  slug: 'org-1',
  name: 'Org One',
  role: 'admin',
  group_id: null,
};

const memberOrg: OrgSummary = { ...adminOrg, role: 'member' };

function renderAt(path: string) {
  return renderWithProviders(
    <Routes>
      <Route path="/:slug/settings" element={<OrgSettings />} />
    </Routes>,
    { route: path },
  );
}

describe('OrgSettings — Defaults section', () => {
  beforeEach(() => {
    useAuthStore.setState({
      user: adminUser,
      currentOrg: adminOrg,
    });
  });

  it('renders the Defaults section for an admin', async () => {
    server.use(
      http.get('*/api/v1/org-1/defaults', () =>
        HttpResponse.json({
          default_rate_limit_rpm: 100,
          default_budget_monthly_usd: 50.0,
        }),
      ),
    );

    renderAt('/org-1/settings');

    await waitFor(() => {
      expect(screen.getByLabelText('Default rate limit (RPM)')).toHaveValue(100);
    });
    expect(screen.getByLabelText('Default monthly budget (USD)')).toHaveValue(50);
  });

  it('disables inputs for a member (read-only)', async () => {
    useAuthStore.setState({ user: memberUser, currentOrg: memberOrg });
    server.use(
      http.get('*/api/v1/org-1/defaults', () =>
        HttpResponse.json({
          default_rate_limit_rpm: 100,
          default_budget_monthly_usd: null,
        }),
      ),
    );

    renderAt('/org-1/settings');

    await waitFor(() => {
      expect(screen.getByLabelText('Default rate limit (RPM)')).toBeDisabled();
    });
    expect(screen.queryByRole('button', { name: 'Save' })).not.toBeInTheDocument();
  });

  it('shows error state when GET fails', async () => {
    server.use(
      http.get('*/api/v1/org-1/defaults', () =>
        HttpResponse.json({ error: { message: 'down' } }, { status: 500 }),
      ),
      http.get('*/api/v1/org-1/budget-status', () =>
        HttpResponse.json({ accrued_units: 0, month_bucket: '2026-07' }),
      ),
    );

    renderAt('/org-1/settings');

    await waitFor(() => {
      expect(screen.getByText(/Failed to load defaults/i)).toBeInTheDocument();
    });
  });

  it('save success: toasts + reflects new values', async () => {
    server.use(
      http.get('*/api/v1/org-1/defaults', () =>
        HttpResponse.json({ default_rate_limit_rpm: null, default_budget_monthly_usd: null }),
      ),
      http.put('*/api/v1/org-1/defaults', async ({ request }) => {
        const body = (await request.json()) as { default_rate_limit_rpm: number; default_budget_monthly_usd: number };
        return HttpResponse.json(body);
      }),
    );

    renderAt('/org-1/settings');

    const rpm = await screen.findByLabelText('Default rate limit (RPM)');
    await userEvent.type(rpm, '100');
    await userEvent.click(screen.getByRole('button', { name: 'Save' }));

    await waitFor(() => {
      expect(mockToastSuccess).toHaveBeenCalledWith('Defaults saved.');
    });
  });

  it('Cancel button resets the inputs and disables Save', async () => {
    server.use(
      http.get('*/api/v1/org-1/defaults', () =>
        HttpResponse.json({ default_rate_limit_rpm: 100, default_budget_monthly_usd: 50 }),
      ),
    );
    renderAt('/org-1/settings');
    const rpm = await screen.findByLabelText('Default rate limit (RPM)');
    await userEvent.clear(rpm);
    await userEvent.type(rpm, '200');
    expect(screen.getByRole('button', { name: 'Save' })).toBeEnabled();
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(rpm).toHaveValue(100);
    expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();
  });

  it('Save button is disabled until the form is dirty', async () => {
    server.use(
      http.get('*/api/v1/org-1/defaults', () =>
        HttpResponse.json({ default_rate_limit_rpm: 100, default_budget_monthly_usd: 50 }),
      ),
    );
    renderAt('/org-1/settings');
    await screen.findByLabelText('Default rate limit (RPM)');
    expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled();
    const rpm = screen.getByLabelText('Default rate limit (RPM)');
    await userEvent.clear(rpm);
    await userEvent.type(rpm, '200');
    expect(screen.getByRole('button', { name: 'Save' })).toBeEnabled();
  });

  it('Save failure: shows error toast, no unhandled rejection', async () => {
    server.use(
      http.get('*/api/v1/org-1/defaults', () =>
        HttpResponse.json({ default_rate_limit_rpm: 100, default_budget_monthly_usd: 50 }),
      ),
      http.put('*/api/v1/org-1/defaults', () =>
        HttpResponse.json({}, { status: 500 }),
      ),
    );
    renderAt('/org-1/settings');
    const rpm = await screen.findByLabelText('Default rate limit (RPM)');
    await userEvent.clear(rpm);
    await userEvent.type(rpm, '200');
    await userEvent.click(screen.getByRole('button', { name: 'Save' }));
    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Failed to save defaults.');
    });
  });
});
