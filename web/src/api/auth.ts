import { apiClient } from './client';
import type { AuthResponse, ChangePasswordRequest, LoginRequest, MeResponse, RefreshResponse, RegisterRequest, AuthConfigResponse, User } from '../types';

export async function login(input: LoginRequest): Promise<AuthResponse> {
  const { data } = await apiClient.post<AuthResponse>('/auth/login', input);
  return data;
}

export async function register(input: RegisterRequest): Promise<AuthResponse> {
  // Send snake_case to match the backend's serde-Deserialize struct.
  // Task 8 wired the backend to accept invite_token in the register body and
  // server-side-accept the invitation in the same transaction — so the
  // previous client-side accept is no longer needed.
  const body: Record<string, unknown> = {
    username: input.username,
    password: input.password,
    email: input.email,
  };
  if (input.inviteToken) {
    body.invite_token = input.inviteToken;
  }
  const { data } = await apiClient.post<AuthResponse>('/auth/register', body);
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

export async function switchOrg(orgSlug: string): Promise<AuthResponse> {
  const { data } = await apiClient.post<AuthResponse>('/me/current-org', { org_slug: orgSlug });
  return data;
}

/**
 * Submit a verification token pulled from the email link. The backend marks
 * the user's email_verified_at; on success returns 204 (no body).
 */
export async function verifyEmail(token: string): Promise<void> {
  await apiClient.post('/auth/verify-email', { token });
}

/**
 * Request the backend re-send the verification email. Safe to call repeatedly
 * — the backend rate-limits and degrades errors to a warn log so the caller
 * doesn't learn whether the email exists.
 */
export async function resendVerification(email: string): Promise<void> {
  await apiClient.post('/auth/resend-verification', { email });
}
