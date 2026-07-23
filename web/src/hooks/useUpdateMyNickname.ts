import { useMutation, useQueryClient } from '@tanstack/react-query';
import { setMyNickname } from '../api/auth';
import { useAuthStore } from '../stores/authStore';

/**
 * Mutation: set or clear the current user's nickname. On success:
 *   - Updates the local `user` state in `useAuthStore` so the UI reflects the
 *     new nickname immediately (the response carries it, no need to wait for
 *     the `/auth/me` refetch).
 *   - Invalidates the `['me']` query so `useAuthBootstrap`'s next mount /
 *     refetch reconciles any other fields that drifted.
 *
 * Mirrors the `AddEmailModal` pattern (thread fresh response fields back into
 * `setUser`) and uses the same React Query key as `useAuthBootstrap` (`['me']`).
 */
export function useUpdateMyNickname() {
  const qc = useQueryClient();
  const setUser = useAuthStore((s) => s.setUser);

  return useMutation({
    mutationFn: (nickname: string) => setMyNickname(nickname),
    onSuccess: (data) => {
      // setUser takes a full `User` (not a patch callback), so rebuild from
      // the previous user to preserve fields the response doesn't echo (e.g.
      // `balance`, `threshold`).
      const prev = useAuthStore.getState().user;
      if (prev) {
        setUser({
          ...prev,
          nickname: data.nickname ?? null,
        });
      }
      qc.invalidateQueries({ queryKey: ['me'] });
    },
  });
}
