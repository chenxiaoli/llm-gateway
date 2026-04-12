import { apiClient } from './client';
import type { AuthResponse, ChangePasswordRequest, LoginRequest, MeResponse, RefreshResponse, RegisterRequest, AuthConfigResponse, User } from '../types';

export async function login(input: LoginRequest): Promise<AuthResponse> {
  const { data } = await apiClient.post<AuthResponse>('/auth/login', input);
  return data;
}

export async function register(input: RegisterRequest): Promise<AuthResponse> {
  const { data } = await apiClient.post<AuthResponse>('/auth/register', input);
  return data;
}

export async function getMe(): Promise<MeResponse> {
  const { data } = await apiClient.get<MeResponse>('/auth/me');
  return data;
}

export async function getAuthConfig(): Promise<AuthConfigResponse> {
  const { data } = await apiClient.get<AuthConfigResponse>('/auth/config');
  return data;
}

export async function refreshToken(input: { refresh_token: string }): Promise<RefreshResponse> {
  const { data } = await apiClient.post<RefreshResponse>('/auth/refresh', input);
  return data;
}

export async function changePassword(input: ChangePasswordRequest): Promise<User> {
  const { data } = await apiClient.post<User>('/auth/change-password', input);
  return data;
}
