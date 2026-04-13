import { create } from 'zustand';
import { useEffect } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { getMe, login as apiLogin, register as apiRegister } from '../api/auth';
import { getToken, setToken, clearToken, setRefreshToken, clearRefreshToken } from '../api/client';
import type { User, LoginRequest, RegisterRequest, AuthResponse } from '../types';

interface AuthState {
  user: User | null;
  isLoading: boolean;
  login: (input: LoginRequest) => Promise<AuthResponse>;
  register: (input: RegisterRequest) => Promise<AuthResponse>;
  logout: () => void;
  setUser: (user: User) => void;
}

export const useAuthStore = create<AuthState>((set) => ({
  user: null,
  isLoading: false,

  login: async (input: LoginRequest) => {
    set({ isLoading: true });
    try {
      const resp = await apiLogin(input);
      setToken(resp.token);
      setRefreshToken(resp.refresh_token);
      const me = await getMe();
      set({ user: { id: me.id, username: me.username, role: me.role }, isLoading: false });
      return resp;
    } catch {
      set({ isLoading: false });
      throw new Error('Login failed');
    }
  },

  register: async (input: RegisterRequest) => {
    set({ isLoading: true });
    try {
      const resp = await apiRegister(input);
      setToken(resp.token);
      setRefreshToken(resp.refresh_token);
      const me = await getMe();
      set({ user: { id: me.id, username: me.username, role: me.role }, isLoading: false });
      return resp;
    } catch {
      set({ isLoading: false });
      throw new Error('Registration failed');
    }
  },

  logout: () => {
    clearToken();
    clearRefreshToken();
    set({ user: null });
    window.location.href = '/console/login';
  },

  setUser: (user: User) => set({ user }),
}));

/**
 * Hook to bootstrap auth state on app load.
 * Call once in App — fetches /auth/me if a token exists.
 */
export function useAuthBootstrap() {
  const setUser = useAuthStore((s) => s.setUser);
  const { data: me, isLoading } = useQuery({
    queryKey: ['me'],
    queryFn: getMe,
    retry: false,
    enabled: !!getToken(),
  });

  const queryClient = useQueryClient();

  // Listen for auth expiry events from the API interceptor
  useEffect(() => {
    const handleExpired = () => {
      queryClient.clear();
    };
    window.addEventListener('auth:expired', handleExpired);
    return () => window.removeEventListener('auth:expired', handleExpired);
  }, [queryClient]);

  // Sync React Query data into Zustand store
  useEffect(() => {
    if (me && !useAuthStore.getState().user) {
      setUser({ id: me.id, username: me.username, role: me.role });
    }
  }, [me, setUser]);

  return { isLoading };
}
