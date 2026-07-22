import { describe, it, expect } from 'vitest';
import { renderWithProviders } from '../test/render';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { screen, waitFor } from '@testing-library/react';
import Models from './Models';
import type { ModelWithProvider } from '../types';

function makeModel(overrides: Partial<ModelWithProvider> = {}): ModelWithProvider {
  return {
    id: 'm1', owner_org_id: null, name: 'test-model',
    model_type: null, pricing_policy_id: null,
    supports_vision: false, supports_tools: false,
    created_at: '2026-01-01T00:00:00Z',
    pricing_policy_name: null, channel_ids: [], channel_names: [],
    ...overrides,
  };
}

// Backend wraps Model in { model, pricing_policy_name, channel_ids, channel_names } — match the actual response shape from listAllModels.
function makeApiItem(overrides: Partial<ModelWithProvider> = {}) {
  const m = makeModel(overrides);
  return {
    model: {
      id: m.id, owner_org_id: m.owner_org_id, name: m.name,
      model_type: m.model_type, pricing_policy_id: m.pricing_policy_id,
      supports_vision: m.supports_vision, supports_tools: m.supports_tools,
      created_at: m.created_at,
    },
    pricing_policy_name: m.pricing_policy_name,
    channel_ids: m.channel_ids,
    channel_names: m.channel_names,
  };
}

describe('Models page capability badges', () => {
  it('displays Vision badge when supports_vision=true', async () => {
    server.use(
      http.get('*/api/v1/test-org/admin/models', () =>
        HttpResponse.json([makeApiItem({ name: 'vision-model', supports_vision: true })]),
      ),
      http.get('*/api/v1/test-org/admin/pricing-policies', () => HttpResponse.json([])),
    );
    renderWithProviders(<Models />, { route: '/test-org/admin/models' });
    await waitFor(() => {
      expect(screen.getByText('vision-model')).toBeInTheDocument();
    }, { timeout: 5000 });
    expect(screen.getByText('Vision')).toBeInTheDocument();
    expect(screen.queryByText('Tools')).not.toBeInTheDocument();
  });

  it('displays Tools badge when supports_tools=true', async () => {
    server.use(
      http.get('*/api/v1/test-org/admin/models', () =>
        HttpResponse.json([makeApiItem({ name: 'tools-model', supports_tools: true })]),
      ),
      http.get('*/api/v1/test-org/admin/pricing-policies', () => HttpResponse.json([])),
    );
    renderWithProviders(<Models />, { route: '/test-org/admin/models' });
    await waitFor(() => {
      expect(screen.getByText('tools-model')).toBeInTheDocument();
    }, { timeout: 5000 });
    expect(screen.getByText('Tools')).toBeInTheDocument();
    expect(screen.queryByText('Vision')).not.toBeInTheDocument();
  });
});
