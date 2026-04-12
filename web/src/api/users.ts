import { apiClient } from './client';
import type { UserResponse, UpdateUserRequest } from '../types';

export async function listUsers(): Promise<UserResponse[]> {
  const { data } = await apiClient.get<UserResponse[]>('/users');
  return data;
}

export async function updateUser(id: string, input: UpdateUserRequest): Promise<UserResponse> {
  const { data } = await apiClient.patch<UserResponse>(`/users/${id}`, input);
  return data;
}

export async function deleteUser(id: string): Promise<void> {
  await apiClient.delete(`/users/${id}`);
}
