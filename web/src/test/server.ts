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
      platform_role: 'platform_admin',
      current_org: {
        id: 'org-1',
        slug: 'default',
        name: 'Default Org',
        role: 'owner',
        group_id: null,
      },
      orgs: [
        {
          id: 'org-1',
          slug: 'default',
          name: 'Default Org',
          role: 'owner',
          group_id: null,
        },
      ],
      allow_registration: true,
    });
  }),
  http.post('*/api/v1/auth/login', () => {
    return HttpResponse.json({
      token: 'test-jwt-token',
      refresh_token: 'test-refresh-jwt-token',
      user: { id: 'user-1', username: 'admin', platform_role: 'platform_admin' },
      current_org: {
        id: 'org-1',
        slug: 'default',
        name: 'Default Org',
        role: 'owner',
        group_id: null,
      },
      orgs: [
        {
          id: 'org-1',
          slug: 'default',
          name: 'Default Org',
          role: 'owner',
          group_id: null,
        },
      ],
    });
  }),
  http.post('*/api/v1/auth/register', () => {
    return HttpResponse.json({
      token: 'test-jwt-token',
      refresh_token: 'test-refresh-jwt-token',
      user: { id: 'user-1', username: 'admin', platform_role: 'platform_admin' },
      current_org: {
        id: 'org-1',
        slug: 'default',
        name: 'Default Org',
        role: 'owner',
        group_id: null,
      },
      orgs: [
        {
          id: 'org-1',
          slug: 'default',
          name: 'Default Org',
          role: 'owner',
          group_id: null,
        },
      ],
    });
  }),
  http.post('*/api/v1/auth/refresh', () => {
    return HttpResponse.json({
      token: 'new-test-jwt-token',
      refresh_token: 'new-test-refresh-jwt-token',
    });
  }),
  http.get('*/api/v1/test-org/keys', () => {
    return HttpResponse.json({ items: [], total: 0, page: 1, page_size: 20 });
  }),
  http.get('*/api/v1/test-org/admin/providers', () => {
    return HttpResponse.json([]);
  }),
  http.get('*/api/v1/test-org/admin/users', () => {
    return HttpResponse.json({ items: [], total: 0, page: 1, page_size: 20 });
  }),
  http.get('*/api/v1/test-org/members', () => {
    return HttpResponse.json([
      {
        user_id: 'user-1',
        username: 'admin',
        role: 'owner',
        group_id: null,
        joined_at: '2026-01-01T00:00:00Z',
      },
    ]);
  }),
  http.post('*/api/v1/test-org/members', async ({ request }) => {
    const body = (await request.json()) as { username?: string; role?: string };
    return HttpResponse.json({
      user_id: 'invited-1',
      username: body.username ?? 'newuser',
      role: body.role ?? 'member',
      group_id: null,
      joined_at: new Date().toISOString(),
    });
  }),
  http.patch('*/api/v1/test-org/members/*', async ({ request }) => {
    const body = (await request.json()) as { role?: string };
    return HttpResponse.json({
      user_id: 'user-1',
      username: 'admin',
      role: body.role ?? 'member',
      group_id: null,
      joined_at: '2026-01-01T00:00:00Z',
    });
  }),
  http.delete('*/api/v1/test-org/members/*', () => {
    return new HttpResponse(null, { status: 204 });
  }),
  http.get('*/api/v1/test-org/admin/settings', () => {
    return HttpResponse.json({
      allow_registration: true,
      server_host: 'http://localhost:8080',
      audit_log_request: true,
      audit_log_response: true,
    });
  }),
  http.patch('*/api/v1/test-org/admin/settings', () => {
    return HttpResponse.json({
      allow_registration: true,
      server_host: 'http://localhost:8080',
      audit_log_request: true,
      audit_log_response: true,
    });
  }),
  http.get('*/api/v1/test-org/usage', () => {
    return HttpResponse.json({ items: [], total: 0, page: 1, page_size: 20 });
  }),
  http.get('*/api/v1/test-org/admin/logs', () => {
    return HttpResponse.json({ items: [], total: 0, page: 1, page_size: 20 });
  }),
  http.get('*/api/v1/version', () => {
    return HttpResponse.json({ version: 'v2.0.0' });
  }),

  http.get('*/api/v1/admin/system-info', () => {
    return HttpResponse.json({
      server_bind_address: '0.0.0.0:8080',
      database_driver: 'postgres',
      rate_limit_window_secs: 60,
      rate_limit_flush_interval_secs: 30,
      upstream_timeout_secs: 30,
      audit_retention_days: 90,
    });
  }),

  http.get('*/api/v1/admin/system-info', () => {
    return HttpResponse.json({
      server_bind_address: '0.0.0.0:8080',
      database_driver: 'postgres',
      rate_limit_window_secs: 60,
      rate_limit_flush_interval_secs: 30,
      upstream_timeout_secs: 30,
      audit_retention_days: 90,
    });
  }),
);
