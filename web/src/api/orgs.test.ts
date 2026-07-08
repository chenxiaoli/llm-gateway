import { describe, it, expect, beforeEach } from 'vitest';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { updateOrg, deleteOrg } from './orgs';
import { setToken } from './client';

beforeEach(() => {
  setToken('test-admin-token');
});

describe('orgs API', () => {
  it('updateOrg sends PATCH with name + slug and returns updated org', async () => {
    server.use(
      http.patch('*/api/v1/test-org', async ({ request }) => {
        const body = (await request.json()) as Record<string, unknown>;
        expect(body.name).toBe('Acme');
        expect(body.slug).toBe('acme');
        return HttpResponse.json({
          id: 'org-1',
          slug: 'acme',
          name: 'Acme',
          role: 'owner',
          group_id: null,
        });
      }),
    );

    const result = await updateOrg({ name: 'Acme', slug: 'acme' });
    expect(result.slug).toBe('acme');
    expect(result.name).toBe('Acme');
  });

  it('updateOrg omits undefined fields from the request body', async () => {
    server.use(
      http.patch('*/api/v1/test-org', async ({ request }) => {
        const body = (await request.json()) as Record<string, unknown>;
        expect(body).not.toHaveProperty('slug');
        expect(body.name).toBe('Renamed');
        return HttpResponse.json({
          id: 'org-1',
          slug: 'test-org',
          name: 'Renamed',
          role: 'admin',
          group_id: null,
        });
      }),
    );

    const result = await updateOrg({ name: 'Renamed' });
    expect(result.name).toBe('Renamed');
    expect(result.slug).toBe('test-org');
  });

  it('deleteOrg sends DELETE with password in body', async () => {
    let receivedPassword: string | null = null;
    server.use(
      http.delete('*/api/v1/test-org', async ({ request }) => {
        const body = (await request.json()) as { password?: string };
        receivedPassword = body.password ?? null;
        return new HttpResponse(null, { status: 204 });
      }),
    );

    await deleteOrg('hunter2');
    expect(receivedPassword).toBe('hunter2');
  });

  it('deleteOrg resolves on 204', async () => {
    server.use(
      http.delete('*/api/v1/test-org', () => new HttpResponse(null, { status: 204 })),
    );
    await expect(deleteOrg('pw')).resolves.toBeUndefined();
  });
});
