import { apiClient, orgPrefix } from './client';
import type { Member, MemberRole } from '../types';

export async function listMembers(): Promise<Member[]> {
  const { data } = await apiClient.get<Member[]>(`${orgPrefix()}/members`);
  return data;
}

export async function inviteMember(req: { username: string; role: MemberRole }): Promise<Member> {
  const { data } = await apiClient.post<Member>(`${orgPrefix()}/members`, req);
  return data;
}

export async function changeMemberRole(userId: string, role: MemberRole): Promise<Member> {
  const { data } = await apiClient.patch<Member>(`${orgPrefix()}/members/${userId}`, { role });
  return data;
}

export async function removeMember(userId: string): Promise<void> {
  await apiClient.delete(`${orgPrefix()}/members/${userId}`);
}
