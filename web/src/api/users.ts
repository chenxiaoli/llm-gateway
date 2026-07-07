import { apiClient, orgPrefix } from './client';
import type { PaginatedResponse, UpdateUserRequest, UserResponse } from '../types';

export async function listUsers(page = 1, pageSize = 20): Promise<PaginatedResponse<UserResponse>> {
  const { data } = await apiClient.get<PaginatedResponse<UserResponse>>(`${orgPrefix()}/admin/users`, {
    params: { page, page_size: pageSize },
  });
  return data;
}

export async function updateUser(id: string, input: UpdateUserRequest): Promise<UserResponse> {
  const { data } = await apiClient.patch<UserResponse>(`${orgPrefix()}/admin/users/${id}`, input);
  return data;
}

export async function deleteUser(id: string): Promise<void> {
  await apiClient.delete(`${orgPrefix()}/admin/users/${id}`);
}
