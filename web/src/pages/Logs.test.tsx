import { describe, it, expect } from 'vitest';
import { renderWithProviders } from '../test/render';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import Logs from './Logs';

const mockLogsResponse = {
  items: [
    {
      id: 'log-1',
      request_id: 'req-abc-123',
      key_id: 'k1',
      model_name: 'minimax-3',
      provider_id: 'p1',
      channel_id: 'ch-c',
      channel_name: 'Channel C',
      protocol: 'openai',
      stream: false,
      status_code: 200,
      latency_ms: 500,
      input_tokens: 10,
      output_tokens: 20,
      created_at: '2026-07-01T00:00:00Z',
      original_model: 'glm-5.2',
      upstream_model: 'minimax-3',
      routes: [
        { model: 'glm-5.2', channel_id: 'ch-a', channel_name: 'Channel A', status_code: 0, error_message: 'Connection refused', latency_ms: 5, started_at: '2026-07-01T00:00:00Z' },
        { model: 'glm-5.2', channel_id: 'ch-b', channel_name: 'Channel B', status_code: 500, error_message: 'Internal Server Error', latency_ms: 150, started_at: '2026-07-01T00:00:01Z' },
        { model: 'minimax-3', channel_id: 'ch-c', channel_name: 'Channel C', status_code: 200, error_message: null, latency_ms: 320, started_at: '2026-07-01T00:00:02Z' },
      ],
    },
  ],
  total: 1,
  page: 1,
  page_size: 20,
};

describe('Logs page', () => {
  it('renders routes column and shows attempt count badge', async () => {
    server.use(
      http.get('*/api/v1/admin/logs', () => HttpResponse.json(mockLogsResponse)),
      http.get('*/api/v1/admin/channels', () => HttpResponse.json([])),
    );

    renderWithProviders(<Logs />, { route: '/admin/logs' });

    await waitFor(() => {
      expect(screen.getByText(/3 routes|3 条路由/)).toBeInTheDocument();
    });
  });

  it('opens routes modal with all attempts on badge click', async () => {
    server.use(
      http.get('*/api/v1/admin/logs', () => HttpResponse.json(mockLogsResponse)),
      http.get('*/api/v1/admin/channels', () => HttpResponse.json([])),
    );

    const user = userEvent.setup();
    renderWithProviders(<Logs />, { route: '/admin/logs' });

    const badge = await screen.findByText(/3 routes|3 条路由/);
    await user.click(badge);

    await waitFor(() => {
      expect(screen.getByText('Connection refused')).toBeInTheDocument();
      expect(screen.getByText('Internal Server Error')).toBeInTheDocument();
    });
  });
});