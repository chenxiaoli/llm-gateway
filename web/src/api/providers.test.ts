import { describe, it, expect, beforeEach } from 'vitest';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { listProviders, createProvider, updateProvider, deleteProvider } from './providers';
import { setToken } from './client';
import type { Provider } from '../types';

const mockProvider: Provider = {
  id: 'prov-1',
  name: 'openai',
  api_key: 'sk-test',
  openai_base_url: 'https://api.openai.com',
  anthropic_base_url: null,
  enabled: true,
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
};

beforeEach(() => {
  setToken('test-admin-token');
});

describe('providers API', () => {
  it('listProviders fetches all providers', async () => {
    server.use(
      http.get('*/api/v1/providers', () => {
        return HttpResponse.json([mockProvider]);
      }),
    );

    const providers = await listProviders();
    expect(providers).toHaveLength(1);
    expect(providers[0].name).toBe('openai');
    expect(providers[0].openai_base_url).toBe('https://api.openai.com');
  });

  it('createProvider sends POST and returns provider', async () => {
    server.use(
      http.post('*/api/v1/providers', async ({ request }) => {
        const body = await request.json() as Record<string, unknown>;
        expect(body.name).toBe('anthropic');
        expect(body.api_key).toBe('sk-ant-test');
        return HttpResponse.json({
          ...mockProvider,
          id: 'prov-2',
          name: 'anthropic',
          api_key: 'sk-ant-test',
          anthropic_base_url: 'https://api.anthropic.com',
          openai_base_url: null,
        });
      }),
    );

    const result = await createProvider({
      name: 'anthropic',
      api_key: 'sk-ant-test',
      anthropic_base_url: 'https://api.anthropic.com',
    });
    expect(result.name).toBe('anthropic');
    expect(result.anthropic_base_url).toBe('https://api.anthropic.com');
  });

  it('updateProvider sends PATCH', async () => {
    server.use(
      http.patch('*/api/v1/providers/prov-1', async ({ request }) => {
        const body = await request.json() as Record<string, unknown>;
        expect(body.enabled).toBe(false);
        return HttpResponse.json({ ...mockProvider, enabled: false });
      }),
    );

    const result = await updateProvider('prov-1', { enabled: false });
    expect(result.enabled).toBe(false);
  });

  it('deleteProvider sends DELETE', async () => {
    let deleted = false;
    server.use(
      http.delete('*/api/v1/providers/prov-1', () => {
        deleted = true;
        return new HttpResponse(null, { status: 204 });
      }),
    );

    await deleteProvider('prov-1');
    expect(deleted).toBe(true);
  });

  it('listProviders returns empty array when no providers', async () => {
    server.use(
      http.get('*/api/v1/providers', () => {
        return HttpResponse.json([]);
      }),
    );

    const providers = await listProviders();
    expect(providers).toEqual([]);
  });
});
