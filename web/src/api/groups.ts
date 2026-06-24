import { adminApiClient } from './client';
import type { Group, CreateGroupRequest, UpdateGroupRequest, DeleteGroupResult } from '../types';

export async function listGroups(): Promise<Group[]> {
  const { data } = await adminApiClient.get<Group[]>('/groups');
  return data;
}

export async function getGroup(id: string): Promise<Group> {
  const { data } = await adminApiClient.get<Group>(`/groups/${id}`);
  return data;
}

export async function createGroup(input: CreateGroupRequest): Promise<Group> {
  const { data } = await adminApiClient.post<Group>('/groups', input);
  return data;
}

export async function updateGroup(id: string, input: UpdateGroupRequest): Promise<Group> {
  const { data } = await adminApiClient.patch<Group>(`/groups/${id}`, input);
  return data;
}

export async function deleteGroup(id: string): Promise<DeleteGroupResult> {
  const { data } = await adminApiClient.delete<DeleteGroupResult>(`/groups/${id}`);
  return data;
}
