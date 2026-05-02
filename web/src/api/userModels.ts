import { apiClient } from './client';
import type { UserModelView } from '../types';

export async function listUserModels(): Promise<UserModelView[]> {
  const { data } = await apiClient.get<UserModelView[]>('/user/models');
  return Array.isArray(data) ? data : [];
}
