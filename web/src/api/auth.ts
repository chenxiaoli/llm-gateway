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

/**
 * Request a password reset email. The backend is intentionally always-204 —
 * it does not leak whether the email is registered. Callers should show a
 * generic "if an account exists, we've sent a link" message regardless.
 */
export async function requestPasswordReset(email: string): Promise<void> {
  await apiClient.post('/auth/password-reset/request', { email });
}

/**
 * Preview a password reset token (from the email link) without consuming it.
 * Returns `{ valid, expires_at }`. Used by the ResetPassword landing page to
 * decide which panel to show before the user types a new password.
 */
export async function previewPasswordReset(
  token: string,
): Promise<{ valid: boolean; expires_at: string | null }> {
  const { data } = await apiClient.get<{ valid: boolean; expires_at: string | null }>(
    '/auth/password-reset/preview',
    { params: { token } },
  );
  return data;
}

/**
 * Submit a new password for a reset token. Consumes the token — a second call
 * with the same token will fail with `reset_consumed`.
 */
export async function confirmPasswordReset(token: string, new_password: string): Promise<void> {
  await apiClient.post('/auth/password-reset/confirm', { token, new_password });
}

/**
 * Set or change the signed-in user's email. The backend mints a fresh
 * verification token + dispatches the verification email (best-effort).
 * Returns the updated MeResponse so the caller can refresh cached state.
 *
 * Rejects with 409 `email_in_use` if another user already owns the address;
 * 400 (no code) on malformed input — the backend's `validate_email` returns
 * a string surfaced via `ApiError::BadRequest`, which carries no `code` field.
 */
export async function setMyEmail(email: string): Promise<MeResponse> {
  const { data } = await apiClient.post<MeResponse>('/auth/me/email', { email });
  return data;
}

/**
 * Set or clear the current user's nickname. Pass an empty/whitespace string
 * to clear (server trims + writes NULL). Validation is server-side: 1–32
 * Unicode scalar values after trim, no control / zero-width characters.
 *
 * Mirrors `setMyEmail` — returns the refreshed `MeResponse` so the caller can
 * push the fresh user fields into the auth store without waiting for the
 * `/auth/me` refetch.
 *
 * Rejects with 400 `invalid_nickname` on validation failure.
 */
export async function setMyNickname(nickname: string): Promise<MeResponse> {
  const { data } = await apiClient.post<MeResponse>('/auth/me/nickname', { nickname });
  return data;
}
