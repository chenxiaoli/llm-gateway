import { describe, it, expect } from 'vitest';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { listPlatformUsers, searchCandidates, setPlatformRole } from './admin';

describe('admin api wrappers', () => {
  it('listPlatformUsers calls GET /admin/platform-users', async () => {
    let called = false;
    server.use(
      http.get('/api/v1/admin/platform-users', () => {
        called = true;
        return HttpResponse.json({ admins: [], candidates: [] });
      }),
    );
    const r = await listPlatformUsers();
    expect(called).toBe(true);
    expect(r.admins).toEqual([]);
  });

  it('searchCandidates encodes the query', async () => {
    let url = '';
    server.use(
      http.get('/api/v1/admin/platform-users', ({ request }) => {
        url = request.url;
        return HttpResponse.json({ admins: [], candidates: [] });
      }),
    );
    await searchCandidates('alice');
    expect(url).toContain('q=alice');
  });

  it('setPlatformRole PATCHes with body', async () => {
    let body: any;
    server.use(
      http.patch('/api/v1/admin/users/u-1/platform-role', async ({ request }) => {
        body = await request.json();
        return HttpResponse.json({ id: 'u-1', platform_role: 'platform_admin' });
      }),
    );
    await setPlatformRole('u-1', 'platform_admin');
    expect(body).toEqual({ platform_role: 'platform_admin' });
  });
});