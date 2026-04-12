import { describe, it, expect, beforeEach } from 'vitest';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { listKeys, createKey, updateKey, deleteKey } from './keys';
import { setToken } from './client';
import type { ApiKey, CreateKeyResponse } from '../types';

const mockKey: ApiKey = {
  id: 'key-1',
  name: 'test-key',
  key_hash: 'abc123',
  rate_limit: 60,
  budget_monthly: 100.0,
  enabled: true,
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
};

const mockCreateResponse: CreateKeyResponse = {
  id: 'key-1',
  name: 'test-key',
  key: 'sk-live-xxxxxxxxxxxx',
  rate_limit: 60,
  budget_monthly: 100.0,
  enabled: true,
  created_at: '2026-01-01T00:00:00Z',
};

beforeEach(() => {
  setToken('test-admin-token');
});

describe('keys API', () => {
  it('listKeys fetches all keys', async () => {
    server.use(
      http.get('*/api/v1/keys', () => {
        return HttpResponse.json([mockKey]);
      }),
    );

    const keys = await listKeys();
    expect(keys).toHaveLength(1);
    expect(keys[0].name).toBe('test-key');
    expect(keys[0].rate_limit).toBe(60);
  });

  it('createKey sends POST and returns new key', async () => {
    server.use(
      http.post('*/api/v1/keys', async ({ request }) => {
        const body = await request.json() as Record<string, unknown>;
        expect(body.name).toBe('new-key');
        return HttpResponse.json(mockCreateResponse);
      }),
    );

    const result = await createKey({ name: 'new-key' });
    expect(result.key).toBe('sk-live-xxxxxxxxxxxx');
    expect(result.id).toBe('key-1');
  });

  it('updateKey sends PATCH with partial data', async () => {
    server.use(
      http.patch('*/api/v1/keys/key-1', async ({ request }) => {
        const body = await request.json() as Record<string, unknown>;
        expect(body.enabled).toBe(false);
        return HttpResponse.json({ ...mockKey, enabled: false });
      }),
    );

    const result = await updateKey('key-1', { enabled: false });
    expect(result.enabled).toBe(false);
  });

  it('deleteKey sends DELETE', async () => {
    let deleted = false;
    server.use(
      http.delete('*/api/v1/keys/key-1', () => {
        deleted = true;
        return new HttpResponse(null, { status: 204 });
      }),
    );

    await deleteKey('key-1');
    expect(deleted).toBe(true);
  });

  it('listKeys returns empty array when no keys', async () => {
    server.use(
      http.get('*/api/v1/keys', () => {
        return HttpResponse.json([]);
      }),
    );

    const keys = await listKeys();
    expect(keys).toEqual([]);
  });
});
