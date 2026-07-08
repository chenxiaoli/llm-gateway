import { create } from 'zustand';
import { useEffect } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { getMe, login as apiLogin, register as apiRegister, switchOrg } from '../api/auth';
import { getToken, setToken, clearToken, setRefreshToken, clearRefreshToken } from '../api/client';
import { queryClient } from '../lib/queryClient';
import { useCurrencyStore } from './currency';
import type { User, OrgSummary, LoginRequest, RegisterRequest, AuthResponse } from '../types';

interface AuthState {
  user: User | null;
  currentOrg: OrgSummary | null;
  orgs: OrgSummary[];
  isLoading: boolean;
  login: (input: LoginRequest) => Promise<AuthResponse>;
  register: (input: RegisterRequest) => Promise<AuthResponse>;
  logout: () => void;
  setUser: (user: User) => void;
  setCurrentOrg: (org: OrgSummary) => Promise<void>;
  refreshOrgs: () => Promise<void>;
}

export const useAuthStore = create<AuthState>((set) => ({
  user: null,
  currentOrg: null,
  orgs: [],
  isLoading: false,

  login: async (input: LoginRequest) => {
    set({ isLoading: true });
    try {
      const resp = await apiLogin(input);
      setToken(resp.token);
      setRefreshToken(resp.refresh_token);
      const me = await getMe();
      set({
        user: { id: me.id, username: me.username, platform_role: me.platform_role },
        currentOrg: me.current_org,
        orgs: me.orgs,
        isLoading: false,
      });
      return resp;
    } catch (err) {
      set({ isLoading: false });
      throw err;
    }
  },

  register: async (input: RegisterRequest) => {
    set({ isLoading: true });
    try {
      const resp = await apiRegister(input);
      setToken(resp.token);
      setRefreshToken(resp.refresh_token);
      const me = await getMe();
      set({
        user: { id: me.id, username: me.username, platform_role: me.platform_role },
        currentOrg: me.current_org,
        orgs: me.orgs,
        isLoading: false,
      });
      return resp;
    } catch (err) {
      set({ isLoading: false });
      throw err;
    }
  },

  logout: () => {
    clearToken();
    clearRefreshToken();
    set({ user: null, currentOrg: null, orgs: [] });
    window.location.href = '/login';
  },

  setUser: (user: User) => set({ user }),

  setCurrentOrg: async (org: OrgSummary) => {
    const resp = await switchOrg(org.slug);
    setToken(resp.token);
    setRefreshToken(resp.refresh_token);
    set({ currentOrg: org });
    queryClient.clear();
  },

  refreshOrgs: async () => {
    const me = await getMe();
    set({
      user: { id: me.id, username: me.username, platform_role: me.platform_role },
      currentOrg: me.current_org,
      orgs: me.orgs,
    });
  },
}));

/**
 * Hook to bootstrap auth state on app load.
 * Call once in App — fetches /auth/me if a token exists.
 */
export function useAuthBootstrap() {
  const setUser = useAuthStore((s) => s.setUser);
  const initCurrency = useCurrencyStore((s) => s.init);
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
      setUser({ id: me.id, username: me.username, platform_role: me.platform_role });
      useAuthStore.setState({ currentOrg: me.current_org, orgs: me.orgs });
    }
  }, [me, setUser]);

  // Initialize currency from auth config
  useEffect(() => {
    initCurrency();
  }, [initCurrency]);

  return { isLoading };
}
