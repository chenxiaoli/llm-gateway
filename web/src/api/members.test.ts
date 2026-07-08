import { describe, it, expect, beforeEach } from 'vitest';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { listMembers, inviteMember, changeMemberRole, removeMember } from './members';
import { setToken } from './client';
import type { Member } from '../types';

const mockMember: Member = {
  user_id: 'user-1',
  username: 'admin',
  role: 'owner',
  group_id: null,
  joined_at: '2026-01-01T00:00:00Z',
};

beforeEach(() => {
  setToken('test-admin-token');
});

describe('members API', () => {
  it('listMembers returns an array', async () => {
    server.use(
      http.get('*/api/v1/test-org/members', () => HttpResponse.json([mockMember])),
    );

    const members = await listMembers();
    expect(members).toHaveLength(1);
    expect(members[0].username).toBe('admin');
    expect(members[0].role).toBe('owner');
  });

  it('inviteMember sends POST with username + role', async () => {
    server.use(
      http.post('*/api/v1/test-org/members', async ({ request }) => {
        const body = (await request.json()) as Record<string, unknown>;
        expect(body.username).toBe('alice');
        expect(body.role).toBe('admin');
        return HttpResponse.json({
          user_id: 'u-alice',
          username: 'alice',
          role: 'admin',
          group_id: null,
          joined_at: '2026-02-01T00:00:00Z',
        });
      }),
    );

    const result = await inviteMember({ username: 'alice', role: 'admin' });
    expect(result.username).toBe('alice');
    expect(result.role).toBe('admin');
  });

  it('changeMemberRole sends PATCH with role', async () => {
    server.use(
      http.patch('*/api/v1/test-org/members/u-alice', async ({ request }) => {
        const body = (await request.json()) as Record<string, unknown>;
        expect(body.role).toBe('member');
        return HttpResponse.json({
          user_id: 'u-alice',
          username: 'alice',
          role: 'member',
          group_id: null,
          joined_at: '2026-02-01T00:00:00Z',
        });
      }),
    );

    const result = await changeMemberRole('u-alice', 'member');
    expect(result.role).toBe('member');
  });

  it('removeMember sends DELETE', async () => {
    let deleted = false;
    server.use(
      http.delete('*/api/v1/test-org/members/u-alice', () => {
        deleted = true;
        return new HttpResponse(null, { status: 204 });
      }),
    );

    await removeMember('u-alice');
    expect(deleted).toBe(true);
  });

  it('listMembers returns empty array when no members', async () => {
    server.use(
      http.get('*/api/v1/test-org/members', () => HttpResponse.json([])),
    );

    const members = await listMembers();
    expect(members).toEqual([]);
  });
});
