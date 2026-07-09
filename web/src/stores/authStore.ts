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
  /**
   * Stashed invitation token captured pre-auth (e.g. by the /accept-invite page,
   * Task 9). Consumed by register(); cleared after use regardless of outcome so a
   * stale token can't loop.
   */
  pendingInviteToken: string | null;
  /**
   * Per-session flag for "user dismissed the add-email banner this login".
   * Reset to false on login/logout/register so a fresh session shows it again.
   * Switching orgs does NOT reset it — see dismissEmailBanner.
   */
  emailBannerDismissed: boolean;
  dismissEmailBanner: () => void;
  login: (input: LoginRequest) => Promise<AuthResponse>;
  register: (input: RegisterRequest) => Promise<AuthResponse>;
  /**
   * Apply an AuthResponse received outside the login/register flow — namely
   * from POST /orgs (onboarding create branch) and POST /invitations/accept
   * (onboarding join branch). Mirrors the login/register pattern: persists
   * the fresh tokens, then re-fetches /auth/me to populate user + currentOrg
   * + orgs + impersonating in one coherent state update. The extra round-trip
   * is intentional — it avoids drift between the partial AuthResponse shape
   * and the full MeResponse (e.g. impersonating is not on AuthResponse).
   */
  applyAuthResponse: (resp: AuthResponse) => Promise<void>;
  logout: () => void;
  setUser: (user: User) => void;
  setCurrentOrg: (org: OrgSummary) => Promise<void>;
  refreshOrgs: () => Promise<void>;
  setPendingInviteToken: (t: string | null) => void;
}

export const useAuthStore = create<AuthState>((set) => ({
  user: null,
  currentOrg: null,
  orgs: [],
  isLoading: false,
  impersonating: false,
  pendingInviteToken: null,
  emailBannerDismissed: false,
  dismissEmailBanner: () => set({ emailBannerDismissed: true }),

  login: async (input: LoginRequest) => {
    set({ isLoading: true });
    try {
      const resp = await apiLogin(input);
      setToken(resp.token);
      setRefreshToken(resp.refresh_token);
      const me = await getMe();
      set({
        user: { id: me.id, username: me.username, platform_role: me.platform_role, email: me.email, email_verified_at: me.email_verified_at },
        currentOrg: me.current_org,
        orgs: me.orgs,
        // Login always lands the user in their own org — never impersonating.
        impersonating: false,
        isLoading: false,
        emailBannerDismissed: false,
      });
      return resp;
    } catch (err) {
      set({ isLoading: false });
      throw err;
    }
  },

  register: async (input: RegisterRequest) => {
    set({ isLoading: true });
    const inviteToken = useAuthStore.getState().pendingInviteToken;
    try {
      // inviteToken travels inside the RegisterRequest body (Task 9) — the
      // backend accepts the invitation in the same transaction (Task 8).
      const resp = await apiRegister({ ...input, inviteToken: inviteToken ?? undefined });
      setToken(resp.token);
      setRefreshToken(resp.refresh_token);
      const me = await getMe();
      set({
        user: { id: me.id, username: me.username, platform_role: me.platform_role, email: me.email, email_verified_at: me.email_verified_at },
        currentOrg: me.current_org,
        orgs: me.orgs,
        // Register always assigns the user to a fresh default org — never impersonating.
        impersonating: false,
        isLoading: false,
        emailBannerDismissed: false,
      });
      return resp;
    } catch (err) {
      set({ isLoading: false });
      throw err;
    } finally {
      // Clear regardless of outcome so a stale token can't loop.
      if (inviteToken) {
        set({ pendingInviteToken: null });
      }
    }
  },

  applyAuthResponse: async (resp: AuthResponse) => {
    setToken(resp.token);
    setRefreshToken(resp.refresh_token);
    const me = await getMe();
    set({
      user: { id: me.id, username: me.username, platform_role: me.platform_role, email: me.email, email_verified_at: me.email_verified_at },
      currentOrg: me.current_org,
      orgs: me.orgs,
      // Onboarding (create or join) always produces a real membership — never
      // a temp/system impersonation row.
      impersonating: false,
      isLoading: false,
      emailBannerDismissed: false,
    });
  },

  logout: () => {
    clearToken();
    clearRefreshToken();
    set({ user: null, currentOrg: null, orgs: [], impersonating: false, emailBannerDismissed: false });
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
      user: { id: me.id, username: me.username, platform_role: me.platform_role, email: me.email, email_verified_at: me.email_verified_at },
      currentOrg: me.current_org,
      orgs: me.orgs,
      impersonating: me.impersonating,
    });
  },

  refreshOrgs: async () => {
    const me = await getMe();
    set({
      user: { id: me.id, username: me.username, platform_role: me.platform_role, email: me.email, email_verified_at: me.email_verified_at },
      currentOrg: me.current_org,
      orgs: me.orgs,
      impersonating: me.impersonating,
    });
  },

  setPendingInviteToken: (t) => set({ pendingInviteToken: t }),
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
 * True when the signed-in user has zero org memberships — i.e. they need to
 * run the onboarding wizard (create their first org) or accept a pending
 * invitation before they can do anything useful. Route guards bounce such
 * users to /onboarding.
 */
export function useNeedsOnboarding(): boolean {
  return useAuthStore((s) => s.user !== null && s.orgs.length === 0);
}

/**
 * Discriminated auth-gate status shared by every route guard / inline gate so
 * the "loading vs. login vs. ok" decision lives in exactly one place.
 *
 * - 'loading': bootstrap in flight, OR we have a token but no user yet — keep
 *   showing a spinner so we don't bounce an authenticated user to /login.
 * - 'login': no token and no user — send to /login.
 * - 'ok': user resolved — render the protected content.
 *
 * Used by RequireAuth (App.tsx) and the inline OnboardingGate (Onboarding.tsx).
 * RequireAdmin keeps its own copy because it layers `isAdminOrAbove` on top.
 */
export type AuthGateStatus = 'loading' | 'login' | 'ok';

export function useAuthGate(): AuthGateStatus {
  const user = useAuthStore((s) => s.user);
  const { isLoading } = useAuthBootstrap();
  if (isLoading) return 'loading';
  if (!user) {
    if (getToken()) return 'loading';
    return 'login';
  }
  return 'ok';
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
      setUser({ id: me.id, username: me.username, platform_role: me.platform_role, email: me.email, email_verified_at: me.email_verified_at });
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
