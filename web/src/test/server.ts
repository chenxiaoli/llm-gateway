import { setupServer } from 'msw/node';
import { http, HttpResponse } from 'msw';

export const server = setupServer(
  http.get('*/api/v1/auth/config', () => {
    return HttpResponse.json({ allow_registration: true });
  }),
  http.get('*/api/v1/auth/me', () => {
    return HttpResponse.json({
      id: 'user-1',
      username: 'admin',
      role: 'admin',
      allow_registration: true,
    });
  }),
  http.post('*/api/v1/auth/login', () => {
    return HttpResponse.json({
      token: 'test-jwt-token',
      refresh_token: 'test-refresh-jwt-token',
      user: { id: 'user-1', username: 'admin', role: 'admin' },
    });
  }),
  http.post('*/api/v1/auth/register', () => {
    return HttpResponse.json({
      token: 'test-jwt-token',
      refresh_token: 'test-refresh-jwt-token',
      user: { id: 'user-1', username: 'admin', role: 'admin' },
    });
  }),
  http.post('*/api/v1/auth/refresh', () => {
    return HttpResponse.json({
      token: 'new-test-jwt-token',
      refresh_token: 'new-test-refresh-jwt-token',
    });
  }),
  http.get('*/api/v1/keys', () => {
    return HttpResponse.json({ items: [], total: 0, page: 1, page_size: 20 });
  }),
  http.get('*/api/v1/providers', () => {
    return HttpResponse.json([]);
  }),
  http.get('*/api/v1/users', () => {
    return HttpResponse.json({ items: [], total: 0, page: 1, page_size: 20 });
  }),
  http.get('*/api/v1/settings', () => {
    return HttpResponse.json({
      allow_registration: true,
      server_host: 'http://localhost:8080',
      audit_log_request: true,
      audit_log_response: true,
    });
  }),
  http.patch('*/api/v1/settings', () => {
    return HttpResponse.json({
      allow_registration: true,
      server_host: 'http://localhost:8080',
      audit_log_request: true,
      audit_log_response: true,
    });
  }),
  http.get('*/api/v1/admin/settings', () => {
    return HttpResponse.json({
      allow_registration: true,
      server_host: 'http://localhost:8080',
      audit_log_request: true,
      audit_log_response: true,
    });
  }),
  http.patch('*/api/v1/admin/settings', () => {
    return HttpResponse.json({
      allow_registration: true,
      server_host: 'http://localhost:8080',
      audit_log_request: true,
      audit_log_response: true,
    });
  }),
  http.get('*/api/v1/usage', () => {
    return HttpResponse.json({ items: [], total: 0, page: 1, page_size: 20 });
  }),
  http.get('*/api/v1/logs', () => {
    return HttpResponse.json({ items: [], total: 0, page: 1, page_size: 20 });
  }),
  http.get('*/api/v1/version', () => {
    return HttpResponse.json({ version: 'v0.9.5' });
  }),

  http.get('*/api/v1/admin/system-info', () => {
    return HttpResponse.json({
      server_bind_address: '0.0.0.0:8080',
      database_driver: 'sqlite',
      rate_limit_window_secs: 60,
      rate_limit_flush_interval_secs: 30,
      upstream_timeout_secs: 30,
      audit_retention_days: 90,
    });
  }),
);
