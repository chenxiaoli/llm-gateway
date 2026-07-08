import { apiClient } from './client';
import { acceptInvitation } from './invitations';
import type { AuthResponse, ChangePasswordRequest, LoginRequest, MeResponse, RefreshResponse, RegisterRequest, AuthConfigResponse, User } from '../types';

export async function login(input: LoginRequest): Promise<AuthResponse> {
  const { data } = await apiClient.post<AuthResponse>('/auth/login', input);
  return data;
}

export async function register(
  input: RegisterRequest,
  inviteToken?: string | null,
): Promise<AuthResponse> {
  const { data } = await apiClient.post<AuthResponse>('/auth/register', input);
  // If an invite token was stashed, immediately accept it after register.
  // The accept response is itself an AuthResponse reflecting the joined org.
  // A failed accept (expired/revoked/already-accepted) gracefully degrades to
  // the register response so the caller can still log the user in to onboarding.
  if (inviteToken) {
    try {
      const acceptData = await acceptInvitation({ token: inviteToken });
      return acceptData;
    } catch (err) {
      console.warn('Invite accept failed after register; continuing to onboarding', err);
    }
  }
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
