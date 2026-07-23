import { describe, it, expect, vi } from 'vitest';
import { Toaster } from 'sonner';
import { renderWithProviders } from '../test/render';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import AutoRoutes from './AutoRoutes';
import type { AutoRouteConfig, ModelWithProvider } from '../types';

function renderAutoRoutes(route = '/test-org/auto-routes') {
  return renderWithProviders(
    <>
      <AutoRoutes />
      <Toaster />
    </>,
    { route },
  );
}

const mockConfigs: AutoRouteConfig[] = [
  {
    id: 'cfg-1',
    name: 'Vision pool',
    config: { model_names: ['gpt-4o', 'claude-3-5-sonnet'] },
    created_by: null,
    created_at: '2026-07-15T00:00:00Z',
  },
];

const mockModels: ModelWithProvider[] = [
  {
    id: 'm1', owner_org_id: null, name: 'gpt-4o',
    model_type: null, pricing_policy_id: null,
    supports_vision: true, supports_tools: true,
    created_at: '2026-01-01T00:00:00Z',
    pricing_policy_name: null, channel_ids: [], channel_names: [],
  },
  {
    id: 'm2', owner_org_id: null, name: 'text-only',
    model_type: null, pricing_policy_id: null,
    supports_vision: false, supports_tools: false,
    created_at: '2026-01-01T00:00:00Z',
    pricing_policy_name: null, channel_ids: [], channel_names: [],
  },
];

// Backend wraps each Model in { model, pricing_policy_name, channel_ids, channel_names };
// listAllModels unwraps them. Mocks must match the wire shape.
function wrapModels(models: ModelWithProvider[]) {
  return models.map((m) => ({
    model: {
      id: m.id, owner_org_id: m.owner_org_id, name: m.name,
      model_type: m.model_type, pricing_policy_id: m.pricing_policy_id,
      supports_vision: m.supports_vision, supports_tools: m.supports_tools,
      created_at: m.created_at,
    },
    pricing_policy_name: m.pricing_policy_name,
    channel_ids: m.channel_ids,
    channel_names: m.channel_names,
  }));
}

describe('AutoRoutes page', () => {
  it('renders empty state when no configs', async () => {
    server.use(
      http.get('*/api/v1/test-org/auto-route-configs', () => HttpResponse.json([])),
      http.get('*/api/v1/test-org/admin/models', () => HttpResponse.json([])),
    );
    renderAutoRoutes();
    await waitFor(() => {
      expect(screen.getByText(/No auto routes configured/i)).toBeInTheDocument();
    }, { timeout: 5000 });
  });

  it('renders existing configs', async () => {
    server.use(
      http.get('*/api/v1/test-org/auto-route-configs', () => HttpResponse.json(mockConfigs)),
      http.get('*/api/v1/test-org/admin/models', () => HttpResponse.json(wrapModels(mockModels))),
    );
    renderAutoRoutes();
    await waitFor(() => {
      expect(screen.getByText('Vision pool')).toBeInTheDocument();
    }, { timeout: 5000 });
    expect(screen.getByText('gpt-4o')).toBeInTheDocument();
    expect(screen.getByText('claude-3-5-sonnet')).toBeInTheDocument();
  });

  it('opens create modal on button click', async () => {
    server.use(
      http.get('*/api/v1/test-org/auto-route-configs', () => HttpResponse.json([])),
      http.get('*/api/v1/test-org/admin/models', () => HttpResponse.json(wrapModels(mockModels))),
    );
    renderAutoRoutes();
    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'Auto Routes' })).toBeInTheDocument();
    }, { timeout: 5000 });
    await userEvent.click(screen.getByRole('button', { name: /New Auto Route/i }));
    expect(screen.getByText('Create Auto Route')).toBeInTheDocument();
  });

  it('validates at least one model is required', async () => {
    server.use(
      http.get('*/api/v1/test-org/auto-route-configs', () => HttpResponse.json([])),
      http.get('*/api/v1/test-org/admin/models', () => HttpResponse.json(wrapModels(mockModels))),
    );
    renderAutoRoutes();
    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'Auto Routes' })).toBeInTheDocument();
    }, { timeout: 5000 });
    await userEvent.click(screen.getByRole('button', { name: /New Auto Route/i }));
    // Type a name but don't select any models — should trigger the validation
    await userEvent.type(screen.getByPlaceholderText('e.g., Vision-capable pool'), 'Pool with no models');
    const createBtn = screen.getAllByRole('button', { name: /^Create$/i })[0];
    await userEvent.click(createBtn);
    await waitFor(() => {
      expect(screen.getByText('Select at least one model')).toBeInTheDocument();
    });
  });

  it('submits create mutation with the entered name and selected models', async () => {
    const createdSpy = vi.fn();
    server.use(
      http.get('*/api/v1/test-org/auto-route-configs', () => HttpResponse.json([])),
      http.get('*/api/v1/test-org/admin/models', () => HttpResponse.json(wrapModels(mockModels))),
      http.post('*/api/v1/test-org/auto-route-configs', async ({ request }) => {
        const body = await request.json();
        createdSpy(body);
        return HttpResponse.json({
          id: 'cfg-new', name: body.name, config: body.config,
          created_by: null, created_at: '2026-07-22T00:00:00Z',
        });
      }),
    );
    renderAutoRoutes();
    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'Auto Routes' })).toBeInTheDocument();
    }, { timeout: 5000 });
    await userEvent.click(screen.getByRole('button', { name: /New Auto Route/i }));
    await userEvent.type(screen.getByPlaceholderText('e.g., Vision-capable pool'), 'My pool');
    // Click the first model in the list — gpt-4o
    await userEvent.click(screen.getByText('gpt-4o'));
    const createBtn = screen.getAllByRole('button', { name: /^Create$/i })[0];
    await userEvent.click(createBtn);
    await waitFor(() => {
      expect(createdSpy).toHaveBeenCalled();
    }, { timeout: 5000 });
    const call = createdSpy.mock.calls[0][0];
    expect(call.name).toBe('My pool');
    expect(call.config.model_names).toEqual(['gpt-4o']);
  });

  it('confirms before delete', async () => {
    server.use(
      http.get('*/api/v1/test-org/auto-route-configs', () => HttpResponse.json(mockConfigs)),
      http.get('*/api/v1/test-org/admin/models', () => HttpResponse.json(wrapModels(mockModels))),
    );
    renderAutoRoutes();
    await waitFor(() => {
      expect(screen.getByText('Vision pool')).toBeInTheDocument();
    }, { timeout: 5000 });
    // Click the delete trash icon
    const deleteBtn = screen.getByRole('button', { name: /Delete Vision pool/i });
    await userEvent.click(deleteBtn);
    expect(screen.getByText(/Delete auto route\?/i)).toBeInTheDocument();
  });
});
