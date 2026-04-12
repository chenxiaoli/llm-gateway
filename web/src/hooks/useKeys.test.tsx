import { describe, it, expect } from 'vitest';
import { renderWithProviders } from '../test/render';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { screen, waitFor } from '@testing-library/react';
import { useKeys, useCreateKey } from './useKeys';
import type { ApiKey } from '../types';

const mockKeys: ApiKey[] = [
  {
    id: 'key-1',
    name: 'prod-key',
    key_hash: 'hash1',
    rate_limit: 60,
    budget_monthly: 100.0,
    enabled: true,
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-01T00:00:00Z',
  },
  {
    id: 'key-2',
    name: 'dev-key',
    key_hash: 'hash2',
    rate_limit: null,
    budget_monthly: null,
    enabled: false,
    created_at: '2026-02-01T00:00:00Z',
    updated_at: '2026-02-01T00:00:00Z',
  },
];

function TestKeysList() {
  const { data, isLoading } = useKeys();
  if (isLoading) return <div>Loading...</div>;
  return (
    <ul>
      {data?.map((k) => <li key={k.id}>{k.name}</li>)}
    </ul>
  );
}

function TestCreateKey({ onCreate }: { onCreate: (key: string) => void }) {
  const createKey = useCreateKey();
  return (
    <button onClick={() => createKey.mutate({ name: 'new-key' }, { onSuccess: (data) => onCreate(data.key) })}>
      Create
    </button>
  );
}

describe('useKeys', () => {
  it('fetches and renders keys', async () => {
    server.use(
      http.get('*/api/v1/keys', () => HttpResponse.json(mockKeys)),
    );

    renderWithProviders(<TestKeysList />);

    expect(screen.getByText('Loading...')).toBeInTheDocument();

    await waitFor(() => {
      expect(screen.getByText('prod-key')).toBeInTheDocument();
      expect(screen.getByText('dev-key')).toBeInTheDocument();
    });
  });

  it('shows empty list when no keys', async () => {
    server.use(
      http.get('*/api/v1/keys', () => HttpResponse.json([])),
    );

    renderWithProviders(<TestKeysList />);

    await waitFor(() => {
      expect(screen.queryByText('Loading...')).not.toBeInTheDocument();
    });

    expect(screen.queryByRole('listitem')).not.toBeInTheDocument();
  });
});

describe('useCreateKey', () => {
  it('creates a key and returns the raw key', async () => {
    server.use(
      http.post('*/api/v1/keys', () =>
        HttpResponse.json({
          id: 'key-new',
          name: 'new-key',
          key: 'sk-live-newkey123',
          rate_limit: null,
          budget_monthly: null,
          enabled: true,
          created_at: '2026-03-01T00:00:00Z',
        }),
      ),
    );

    let createdKeyValue = '';
    renderWithProviders(
      <TestCreateKey onCreate={(key) => { createdKeyValue = key; }} />,
    );

    screen.getByRole('button', { name: 'Create' }).click();

    await waitFor(() => {
      expect(createdKeyValue).toBe('sk-live-newkey123');
    });
  });
});
