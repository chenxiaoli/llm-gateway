import { apiClient, orgPrefix } from './client';
import type { Group, CreateGroupRequest, UpdateGroupRequest, DeleteGroupResult, PaginatedResponse } from '../types';

export async function listGroups(page = 1, pageSize = 20): Promise<PaginatedResponse<Group>> {
  const { data } = await apiClient.get<PaginatedResponse<Group>>(`${orgPrefix()}/admin/groups`, {
    params: { page, page_size: pageSize },
  });
  return data;
}

export async function getGroup(id: string): Promise<Group> {
  const { data } = await apiClient.get<Group>(`${orgPrefix()}/admin/groups/${id}`);
  return data;
}

export async function createGroup(input: CreateGroupRequest): Promise<Group> {
  const { data } = await apiClient.post<Group>(`${orgPrefix()}/admin/groups`, input);
  return data;
}

export async function updateGroup(id: string, input: UpdateGroupRequest): Promise<Group> {
  const { data } = await apiClient.patch<Group>(`${orgPrefix()}/admin/groups/${id}`, input);
  return data;
}

export async function deleteGroup(id: string): Promise<DeleteGroupResult> {
  const { data } = await apiClient.delete<DeleteGroupResult>(`${orgPrefix()}/admin/groups/${id}`);
  return data;
}
