import { apiClient, orgPrefix } from './client';
import type {
  Account,
  AccountBalanceResponse,
  Member,
  MemberRole,
} from '../types';

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

// --- Account actions (per-membership balance management) ---
// These hit /admin/members/{user_id}/* (note the /admin/ prefix — these are
// admin-only even though list/invite/remove live at /members/* without it).

export async function getMemberBalance(
  userId: string,
  page = 1,
  pageSize = 20,
): Promise<AccountBalanceResponse> {
  const { data } = await apiClient.get<AccountBalanceResponse>(
    `${orgPrefix()}/admin/members/${userId}/balance`,
    { params: { page, page_size: pageSize } },
  );
  return data;
}

export async function rechargeMember(
  userId: string,
  data: { amount: number; description?: string },
): Promise<Account> {
  const { data: resp } = await apiClient.post<Account>(
    `${orgPrefix()}/admin/members/${userId}/recharge`,
    { ...data, type: 'credit' as const },
  );
  return resp;
}

export async function adjustMemberBalance(
  userId: string,
  data: {
    type: 'credit_adjustment' | 'debit_refund';
    amount: number;
    description?: string;
  },
): Promise<Account> {
  const { data: resp } = await apiClient.post<Account>(
    `${orgPrefix()}/admin/members/${userId}/adjust`,
    data,
  );
  return resp;
}

export async function setMemberThreshold(
  userId: string,
  threshold: number,
): Promise<Account> {
  const { data: resp } = await apiClient.patch<Account>(
    `${orgPrefix()}/admin/members/${userId}/threshold`,
    { threshold },
  );
  return resp;
}

// Update member — body is partial {role?, enabled?, group_id?}.
export async function updateMember(
  userId: string,
  data: {
    role?: MemberRole;
    enabled?: boolean;
    group_id?: string | null;
  },
): Promise<Member> {
  const { data: resp } = await apiClient.patch<Member>(
    `${orgPrefix()}/members/${userId}`,
    data,
  );
  return resp;
}
