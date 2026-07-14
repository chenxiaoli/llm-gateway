import type { User } from '../types';

/**
 * Pick the most user-friendly identifier available. Username takes priority
 * (legacy users + anyone who sets one later); email is the fallback for
 * email-only sign-ups. Returns empty string if neither is set — callers
 * should handle that case explicitly (e.g. show "Unnamed user").
 */
export function displayName(
  user: Pick<User, 'username' | 'email'>,
): string {
  if (user.username && user.username.length > 0) return user.username;
  return user.email ?? '';
}
