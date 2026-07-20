import type { User } from '../types';

/**
 * Pick the most user-friendly identifier available. Priority:
 *   nickname → username → email → ""
 *
 * `nickname` is the user-chosen friendly name (set via /profile).
 * `username` is set by legacy users (email-auth made it optional).
 * `email` is the always-present fallback for email-only sign-ups.
 *
 * Callers should handle the empty-string case explicitly (e.g. show
 * "Unnamed user").
 */
export function displayName(
  user: Pick<User, 'nickname' | 'username' | 'email'>,
): string {
  if (user.nickname && user.nickname.length > 0) return user.nickname;
  if (user.username && user.username.length > 0) return user.username;
  return user.email ?? '';
}
