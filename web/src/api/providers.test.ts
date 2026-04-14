import { describe, it, expect, beforeEach } from 'vitest';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { listProviders, createProvider, updateProvider, deleteProvider, listChannelModels, createChannelModel, updateChannelModel, deleteChannelModel } from './providers';
import { setToken } from './client';
import type { Provider, ChannelModel } from '../types';

const mockProvider: Provider = {
  id: 'prov-1',
  name: 'openai',
  base_url: 'https://api.openai.com',
  endpoints: '{"openai":"https://api.openai.com","anthropic":null}',
  enabled: true,
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-01T00:00:00Z',
};

const mockChannelModel: ChannelModel = {
  id: 'cm-1',
  channel_id: 'ch-1',
  model_id: 'mod-1',
  upstream_model_name: 'gpt-4o-deploy',
  priority_override: 5,
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
    expect(providers[0].base_url).toBe('https://api.openai.com');
  });

  it('createProvider sends POST and returns provider', async () => {
    server.use(
      http.post('*/api/v1/providers', async ({ request }) => {
        const body = await request.json() as Record<string, unknown>;
        expect(body.name).toBe('anthropic');
        return HttpResponse.json({
          ...mockProvider,
          id: 'prov-2',
          name: 'anthropic',
          base_url: 'https://api.anthropic.com',
          endpoints: '{"openai":null,"anthropic":"https://api.anthropic.com"}',
        });
      }),
    );

    const result = await createProvider({
      name: 'anthropic',
      base_url: 'https://api.anthropic.com',
      endpoints: '{"openai":null,"anthropic":"https://api.anthropic.com"}',
    });
    expect(result.name).toBe('anthropic');
    expect(result.base_url).toBe('https://api.anthropic.com');
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

describe('channel models API', () => {
  it('listChannelModels fetches channel models for provider', async () => {
    server.use(
      http.get('*/api/v1/providers/prov-1/channel-models', () => {
        return HttpResponse.json([mockChannelModel]);
      }),
    );

    const results = await listChannelModels('prov-1');
    expect(results).toHaveLength(1);
    expect(results[0].upstream_model_name).toBe('gpt-4o-deploy');
    expect(results[0].channel_id).toBe('ch-1');
  });

  it('createChannelModel sends POST and returns channel model', async () => {
    server.use(
      http.post('*/api/v1/providers/prov-1/channel-models', async ({ request }) => {
        const body = await request.json() as Record<string, unknown>;
        expect(body.channel_id).toBe('ch-1');
        expect(body.model_id).toBe('mod-1');
        expect(body.upstream_model_name).toBe('gpt-4o-custom');
        return HttpResponse.json({
          ...mockChannelModel,
          upstream_model_name: 'gpt-4o-custom',
        });
      }),
    );

    const result = await createChannelModel('prov-1', {
      channel_id: 'ch-1',
      model_id: 'mod-1',
      upstream_model_name: 'gpt-4o-custom',
    });
    expect(result.upstream_model_name).toBe('gpt-4o-custom');
  });

  it('updateChannelModel sends PATCH', async () => {
    server.use(
      http.patch('*/api/v1/channel-models/cm-1', async ({ request }) => {
        const body = await request.json() as Record<string, unknown>;
        expect(body.upstream_model_name).toBe('new-deploy-name');
        expect(body.enabled).toBe(false);
        return HttpResponse.json({
          ...mockChannelModel,
          upstream_model_name: 'new-deploy-name',
          enabled: false,
        });
      }),
    );

    const result = await updateChannelModel('cm-1', {
      upstream_model_name: 'new-deploy-name',
      enabled: false,
    });
    expect(result.upstream_model_name).toBe('new-deploy-name');
    expect(result.enabled).toBe(false);
  });

  it('deleteChannelModel sends DELETE', async () => {
    let deleted = false;
    server.use(
      http.delete('*/api/v1/channel-models/cm-1', () => {
        deleted = true;
        return new HttpResponse(null, { status: 204 });
      }),
    );

    await deleteChannelModel('cm-1');
    expect(deleted).toBe(true);
  });

  it('listChannelModels returns empty array when no mappings', async () => {
    server.use(
      http.get('*/api/v1/providers/prov-1/channel-models', () => {
        return HttpResponse.json([]);
      }),
    );

    const results = await listChannelModels('prov-1');
    expect(results).toEqual([]);
  });
});
