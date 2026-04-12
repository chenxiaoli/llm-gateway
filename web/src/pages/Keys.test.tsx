import { describe, it, expect } from 'vitest';
import { renderWithProviders } from '../test/render';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import Keys from './Keys';
import type { ApiKey } from '../types';

const mockKeys: ApiKey[] = [
  {
    id: 'key-1',
    name: 'production',
    key_hash: 'hash1',
    rate_limit: 60,
    budget_monthly: 50.0,
    enabled: true,
    created_at: '2026-01-15T00:00:00Z',
    updated_at: '2026-01-15T00:00:00Z',
  },
  {
    id: 'key-2',
    name: 'staging',
    key_hash: 'hash2',
    rate_limit: null,
    budget_monthly: null,
    enabled: false,
    created_at: '2026-02-20T00:00:00Z',
    updated_at: '2026-02-20T00:00:00Z',
  },
];

describe('Keys page', () => {
  it('renders keys table', async () => {
    server.use(
      http.get('*/api/v1/keys', () => HttpResponse.json({ items: mockKeys, total: 2, page: 1, page_size: 20 })),
    );

    renderWithProviders(<Keys />, { route: '/console/keys' });

    await waitFor(() => {
      expect(screen.getByText('production')).toBeInTheDocument();
      expect(screen.getByText('staging')).toBeInTheDocument();
    });

    expect(screen.getByText('Active')).toBeInTheDocument();
    expect(screen.getByText('Disabled')).toBeInTheDocument();
    expect(screen.getByText('60')).toBeInTheDocument();
    expect(screen.getByText('$50.00')).toBeInTheDocument();
    const unlimiteds = screen.getAllByText('Unlimited');
    expect(unlimiteds).toHaveLength(2);
  });

  it('opens create key modal', async () => {
    server.use(
      http.get('*/api/v1/keys', () => HttpResponse.json({ items: [], total: 0, page: 1, page_size: 20 })),
    );

    renderWithProviders(<Keys />, { route: '/console/keys' });

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'API Keys' })).toBeInTheDocument();
    }, { timeout: 5000 });

    await userEvent.click(screen.getByRole('button', { name: /Create Key/i }));

    expect(screen.getByText('Create API Key')).toBeInTheDocument();
    const rateLabels = screen.getAllByText('Rate Limit (RPM)');
    expect(rateLabels.length).toBeGreaterThanOrEqual(1);
  });

  it('creates a key and shows the raw key', async () => {
    server.use(
      http.get('*/api/v1/keys', () => HttpResponse.json({ items: [], total: 0, page: 1, page_size: 20 })),
      http.post('*/api/v1/keys', () =>
        HttpResponse.json({
          id: 'key-new',
          name: 'my-new-key',
          key: 'sk-live-abcdef123456',
          rate_limit: null,
          budget_monthly: null,
          enabled: true,
          created_at: '2026-03-01T00:00:00Z',
        }),
      ),
    );

    renderWithProviders(<Keys />, { route: '/console/keys' });

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'API Keys' })).toBeInTheDocument();
    }, { timeout: 5000 });

    await userEvent.click(screen.getByRole('button', { name: /Create Key/i }));

    await waitFor(() => {
      expect(screen.getByLabelText('Name')).toBeInTheDocument();
    }, { timeout: 5000 });

    await userEvent.type(screen.getByLabelText('Name'), 'my-new-key');
    const createBtn = await screen.findByRole('button', { name: /^Create$/ });
    await userEvent.click(createBtn);

    await waitFor(() => {
      expect(screen.getByText(/Save this key now/)).toBeInTheDocument();
    }, { timeout: 10000 });
    expect(screen.getByText('sk-live-abcdef123456')).toBeInTheDocument();
  }, 15000);

  it('shows empty table when no keys', async () => {
    server.use(
      http.get('*/api/v1/keys', () => HttpResponse.json({ items: [], total: 0, page: 1, page_size: 20 })),
    );

    renderWithProviders(<Keys />, { route: '/console/keys' });

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'API Keys' })).toBeInTheDocument();
    }, { timeout: 5000 });
    expect(screen.queryByText('production')).not.toBeInTheDocument();
  });
});
