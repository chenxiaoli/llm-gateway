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
  /**
   * True when the current membership is a temp/system-created row, i.e. a
   * platform_admin is operating in an org they don't really belong to. Drives
   * the ImpersonationBanner.
   */
  impersonating: boolean;
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
  impersonating: false,

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
        // Login always lands the user in their own org — never impersonating.
        impersonating: false,
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
        // Register always assigns the user to a fresh default org — never impersonating.
        impersonating: false,
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
    set({ user: null, currentOrg: null, orgs: [], impersonating: false });
    window.location.href = '/login';
  },

  setUser: (user: User) => set({ user }),

  setCurrentOrg: async (org: OrgSummary) => {
    const resp = await switchOrg(org.slug);
    setToken(resp.token);
    setRefreshToken(resp.refresh_token);
    set({ currentOrg: org });
    queryClient.clear();
    // switchOrg returns AuthResponse (no impersonating field), so re-fetch /me
    // to learn whether the new current_org is a temp/system membership row.
    const me = await getMe();
    set({
      user: { id: me.id, username: me.username, platform_role: me.platform_role },
      currentOrg: me.current_org,
      orgs: me.orgs,
      impersonating: me.impersonating,
    });
  },

  refreshOrgs: async () => {
    const me = await getMe();
    set({
      user: { id: me.id, username: me.username, platform_role: me.platform_role },
      currentOrg: me.current_org,
      orgs: me.orgs,
      impersonating: me.impersonating,
    });
  },
}));

/**
 * True when the current user may attach a catalog entry to the current org
 * (admin/owner of the org, or a platform_admin). Parallel to the backend
 * `can_create_org_catalog` helper in crates/org/src/access.rs — keep the
 * two in sync if the policy diverges.
 */
export function useCanCreateOrgCatalog(): boolean {
  return useAuthStore((s) =>
    s.currentOrg?.role === 'admin'
    || s.currentOrg?.role === 'owner'
    || s.user?.platform_role === 'platform_admin',
  );
}

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
      useAuthStore.setState({
        currentOrg: me.current_org,
        orgs: me.orgs,
        impersonating: me.impersonating,
      });
    }
  }, [me, setUser]);

  // Initialize currency from auth config
  useEffect(() => {
    initCurrency();
  }, [initCurrency]);

  return { isLoading };
}
