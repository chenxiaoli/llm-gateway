import { createContext, useContext, useEffect } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { getMe, login as apiLogin, register as apiRegister } from '../api/auth';
import { getToken, setToken, clearToken, setRefreshToken, clearRefreshToken } from '../api/client';
import type { User, LoginRequest, RegisterRequest, AuthResponse } from '../types';

interface AuthContextValue {
  user: User | undefined;
  isLoading: boolean;
  login: (input: LoginRequest) => Promise<AuthResponse>;
  register: (input: RegisterRequest) => Promise<AuthResponse>;
  logout: () => void;
}

const AuthContext = createContext<AuthContextValue | null>(null);

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const queryClient = useQueryClient();

  // Listen for auth expiry events from the API interceptor
  useEffect(() => {
    const handleExpired = () => {
      queryClient.clear();
    };
    window.addEventListener('auth:expired', handleExpired);
    return () => window.removeEventListener('auth:expired', handleExpired);
  }, [queryClient]);

  const { data: me, isLoading } = useQuery({
    queryKey: ['me'],
    queryFn: getMe,
    retry: false,
    enabled: !!getToken(),
  });

  const login = async (input: LoginRequest) => {
    const resp = await apiLogin(input);
    setToken(resp.token);
    setRefreshToken(resp.refresh_token);
    const me = await getMe();
    queryClient.setQueryData(['me'], me);
    return resp;
  };

  const register = async (input: RegisterRequest) => {
    const resp = await apiRegister(input);
    setToken(resp.token);
    setRefreshToken(resp.refresh_token);
    const me = await getMe();
    queryClient.setQueryData(['me'], me);
    return resp;
  };

  const logout = () => {
    clearToken();
    clearRefreshToken();
    queryClient.clear();
    window.location.href = '/console/login';
  };

  const user: User | undefined = me
    ? { id: me.id, username: me.username, role: me.role }
    : undefined;

  return (
    <AuthContext.Provider value={{ user, isLoading, login, register, logout }}>
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth() {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error('useAuth must be used within AuthProvider');
  return ctx;
}
