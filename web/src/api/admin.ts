import { adminApiClient } from './client';

export interface PlatformUserBrief {
  id: string;
  username: string | null;
  email: string | null;
  platform_role: 'platform_admin' | null;
}

export interface PlatformUsersResponse {
  admins: PlatformUserBrief[];
  candidates?: PlatformUserBrief[];
}

export async function listPlatformUsers(query?: string): Promise<PlatformUsersResponse> {
  const config = query ? { params: { q: query } } : undefined;
  const { data } = await adminApiClient.get<PlatformUsersResponse>('/platform-users', config);
  return data;
}

export async function searchCandidates(query: string): Promise<PlatformUserBrief[]> {
  const { data } = await adminApiClient.get<PlatformUsersResponse>('/platform-users', {
    params: { q: query },
  });
  return data.candidates ?? [];
}

export async function setPlatformRole(
  userId: string,
  platformRole: 'platform_admin' | null,
): Promise<PlatformUserBrief> {
  const { data } = await adminApiClient.patch<PlatformUserBrief>(
    `/users/${userId}/platform-role`,
    { platform_role: platformRole },
  );
  return data;
}