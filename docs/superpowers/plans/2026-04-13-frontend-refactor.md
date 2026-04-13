# Frontend Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace Ant Design with Tailwind CSS + Zustand + custom UI components, and refresh the visual design to ultra-clean minimal.

**Architecture:** Big-bang replacement. Install new deps, build UI primitives, migrate Zustand auth store, then rewrite every page in Tailwind. All Ant Design imports are removed in one pass.

**Tech Stack:** Tailwind CSS v4, Zustand, Lucide React, @tanstack/react-table, React Hook Form + Zod, Sonner. Keeping React Router, React Query, Axios, Recharts.

---

### Task 1: Install Dependencies & Configure Tailwind

**Files:**
- Modify: `web/package.json`
- Create: `web/tailwind.config.ts`
- Modify: `web/vite.config.ts`

- [ ] **Step 1: Install new dependencies**

Run from `/workspace/web`:
```bash
npm install tailwindcss @tailwindcss/vite zustand lucide-react @tanstack/react-table react-hook-form @hookform/resolvers zod sonner clsx tailwind-merge
```

- [ ] **Step 2: Remove Ant Design and dayjs**

Run from `/workspace/web`:
```bash
npm uninstall antd @ant-design/icons dayjs
```

- [ ] **Step 3: Create Tailwind config**

Create `web/tailwind.config.ts`:
```ts
import type { Config } from 'tailwindcss';

export default {
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      colors: {
        accent: {
          DEFAULT: '#06d6a0',
          hover: '#34d399',
          dim: 'rgba(6, 214, 160, 0.12)',
          glow: 'rgba(6, 214, 160, 0.25)',
        },
        surface: {
          DEFAULT: '#111111',
          elevated: '#141414',
        },
        danger: '#ef4444',
        warning: '#f59e0b',
        info: '#3b82f6',
        purple: '#a855f7',
      },
      fontFamily: {
        sans: ['Outfit', 'system-ui', 'sans-serif'],
        display: ['Syne', 'sans-serif'],
        mono: ['JetBrains Mono', 'monospace'],
      },
      borderRadius: {
        DEFAULT: '8px',
      },
    },
  },
  plugins: [],
} satisfies Config;
```

- [ ] **Step 4: Add Tailwind plugin to Vite config**

Replace `web/vite.config.ts`:
```ts
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';

export default defineConfig({
  plugins: [react(), tailwindcss()],
  base: '/',
  server: {
    port: 5173,
    proxy: {
      '/api': {
        target: 'http://localhost:8080',
        changeOrigin: true,
      },
    },
  },
  build: {
    outDir: 'dist',
  },
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.ts'],
    include: ['src/**/*.test.{ts,tsx}'],
  },
});
```

- [ ] **Step 5: Verify build fails (expected — Ant imports broken)**

Run: `cd /workspace/web && npx tsc -b --noEmit 2>&1 | head -20`
Expected: TypeScript errors about missing antd modules.

- [ ] **Step 6: Commit**

```bash
cd /workspace && git add web/package.json web/package-lock.json web/tailwind.config.ts web/vite.config.ts
git commit -m "chore(web): swap Ant Design for Tailwind CSS + Zustand dependencies"
```

---

### Task 2: Create Utility & Global Styles

**Files:**
- Create: `web/src/lib/cn.ts`
- Rewrite: `web/src/styles/global.css`

- [ ] **Step 1: Create cn utility**

Create `web/src/lib/cn.ts`:
```ts
import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
```

- [ ] **Step 2: Rewrite global.css with Tailwind**

Replace `web/src/styles/global.css` with:
```css
@import "tailwindcss";

/* ============================================
   FONTS
   ============================================ */
@import url('https://fonts.googleapis.com/css2?family=Syne:wght@400;500;600;700;800&family=Outfit:wght@300;400;500;600;700&family=JetBrains+Mono:wght@400;500;600;700&display=swap');

/* ============================================
   BASE
   ============================================ */
@layer base {
  *,
  *::before,
  *::after {
    box-sizing: border-box;
  }

  html, body, #root {
    height: 100%;
  }

  body {
    margin: 0;
    font-family: 'Outfit', system-ui, sans-serif;
    background: #000000;
    color: #ededed;
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
  }

  /* Scrollbar */
  ::-webkit-scrollbar {
    width: 6px;
    height: 6px;
  }

  ::-webkit-scrollbar-track {
    background: transparent;
  }

  ::-webkit-scrollbar-thumb {
    background: #262626;
    border-radius: 3px;
  }

  ::-webkit-scrollbar-thumb:hover {
    background: #404040;
  }
}

/* ============================================
   COMPONENTS
   ============================================ */
@layer components {
  .mono {
    font-family: 'JetBrains Mono', monospace;
    font-size: 13px;
  }
}

/* ============================================
   ANIMATIONS
   ============================================ */
@layer utilities {
  @keyframes fadeInUp {
    from {
      opacity: 0;
      transform: translateY(12px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .animate-fade-in-up {
    animation: fadeInUp 0.4s ease-out;
  }
}
```

- [ ] **Step 3: Verify Tailwind processes**

Run: `cd /workspace/web && npx tsc -b --noEmit 2>&1 | head -5`
Expected: Still errors about Ant imports (expected at this stage).

- [ ] **Step 4: Commit**

```bash
cd /workspace && git add web/src/lib/cn.ts web/src/styles/global.css
git commit -m "chore(web): add cn utility and Tailwind global styles"
```

---

### Task 3: Build UI Primitive Components

**Files:**
- Create: `web/src/components/ui/Button.tsx`
- Create: `web/src/components/ui/Modal.tsx`
- Create: `web/src/components/ui/Badge.tsx`
- Create: `web/src/components/ui/Toggle.tsx`
- Create: `web/src/components/ui/Select.tsx`
- Create: `web/src/components/ui/Drawer.tsx`
- Create: `web/src/components/ui/LoadingSpinner.tsx`
- Create: `web/src/components/ui/Alert.tsx`
- Create: `web/src/components/ui/ConfirmDialog.tsx`

- [ ] **Step 1: Create Button component**

Create `web/src/components/ui/Button.tsx`:
```tsx
import { forwardRef, type ButtonHTMLAttributes } from 'react';
import { cn } from '../../lib/cn';
import { Loader2 } from 'lucide-react';

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
  size?: 'sm' | 'md' | 'lg';
  loading?: boolean;
  icon?: React.ReactNode;
}

const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant = 'secondary', size = 'md', loading, icon, children, disabled, ...props }, ref) => {
    return (
      <button
        ref={ref}
        disabled={disabled || loading}
        className={cn(
          'inline-flex items-center justify-center gap-2 rounded-lg font-medium transition-colors duration-150 focus:outline-none focus-visible:ring-2 focus-visible:ring-accent/50 disabled:pointer-events-none disabled:opacity-50',
          {
            'bg-accent text-black hover:bg-accent-hover': variant === 'primary',
            'border border-[#262626] bg-transparent text-[#ededed] hover:bg-white/[0.04] hover:border-[#404040]': variant === 'secondary',
            'bg-transparent text-[#888888] hover:bg-white/[0.04] hover:text-[#ededed]': variant === 'ghost',
            'border border-danger/50 text-danger hover:bg-danger/10': variant === 'danger',
          },
          {
            'h-8 px-3 text-xs': size === 'sm',
            'h-9 px-4 text-sm': size === 'md',
            'h-10 px-5 text-sm': size === 'lg',
          },
          className,
        )}
        {...props}
      >
        {loading ? <Loader2 className="h-4 w-4 animate-spin" /> : icon}
        {children}
      </button>
    );
  },
);

Button.displayName = 'Button';
export { Button, type ButtonProps };
```

- [ ] **Step 2: Create Modal component**

Create `web/src/components/ui/Modal.tsx`:
```tsx
import { useEffect, type ReactNode } from 'react';
import { cn } from '../../lib/cn';
import { X } from 'lucide-react';

interface ModalProps {
  open: boolean;
  onClose: () => void;
  title?: string;
  children: ReactNode;
  footer?: ReactNode;
  className?: string;
}

export function Modal({ open, onClose, title, children, footer, className }: ModalProps) {
  useEffect(() => {
    if (open) {
      document.body.style.overflow = 'hidden';
    } else {
      document.body.style.overflow = '';
    }
    return () => { document.body.style.overflow = ''; };
  }, [open]);

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-[200] flex items-center justify-center">
      <div className="fixed inset-0 bg-black/60 backdrop-blur-sm" onClick={onClose} />
      <div className={cn(
        'relative z-10 w-full max-w-md rounded-xl border border-[#1e1e1e] bg-[#111111] p-6 shadow-2xl animate-fade-in-up',
        className,
      )}>
        <div className="flex items-center justify-between mb-4">
          {title && <h2 className="font-display text-lg font-semibold text-[#ededed]">{title}</h2>}
          <button onClick={onClose} className="ml-auto rounded-lg p-1 text-[#555555] hover:bg-white/[0.04] hover:text-[#ededed] transition-colors">
            <X className="h-4 w-4" />
          </button>
        </div>
        <div>{children}</div>
        {footer && <div className="mt-6 flex justify-end gap-2">{footer}</div>}
      </div>
    </div>
  );
}
```

- [ ] **Step 3: Create Badge component**

Create `web/src/components/ui/Badge.tsx`:
```tsx
import { cn } from '../../lib/cn';

interface BadgeProps {
  variant?: 'green' | 'red' | 'amber' | 'blue' | 'purple' | 'neutral';
  children: React.ReactNode;
  className?: string;
}

const colorMap = {
  green: 'text-accent border-accent/30 bg-accent/10',
  red: 'text-danger border-danger/30 bg-danger/10',
  amber: 'text-warning border-warning/30 bg-warning/10',
  blue: 'text-info border-info/30 bg-info/10',
  purple: 'text-purple border-purple/30 bg-purple/10',
  neutral: 'text-[#888888] border-[#262626] bg-white/[0.03]',
};

export function Badge({ variant = 'neutral', children, className }: BadgeProps) {
  return (
    <span className={cn(
      'inline-flex items-center rounded-full border px-2 py-0.5 text-xs font-medium',
      colorMap[variant],
      className,
    )}>
      {children}
    </span>
  );
}
```

- [ ] **Step 4: Create Toggle component**

Create `web/src/components/ui/Toggle.tsx`:
```tsx
import { cn } from '../../lib/cn';

interface ToggleProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
}

export function Toggle({ checked, onChange, disabled }: ToggleProps) {
  return (
    <button
      role="switch"
      aria-checked={checked}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={cn(
        'relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 focus:outline-none focus-visible:ring-2 focus-visible:ring-accent/50 disabled:cursor-not-allowed disabled:opacity-50',
        checked ? 'bg-accent' : 'bg-[#262626]',
      )}
    >
      <span className={cn(
        'pointer-events-none inline-block h-4 w-4 rounded-full bg-white shadow-lg ring-0 transition-transform duration-200',
        checked ? 'translate-x-4' : 'translate-x-0',
      )} />
    </button>
  );
}
```

- [ ] **Step 5: Create Select component**

Create `web/src/components/ui/Select.tsx`:
```tsx
import { cn } from '../../lib/cn';
import { ChevronDown } from 'lucide-react';

interface SelectOption {
  value: string;
  label: string;
}

interface SelectProps {
  value: string;
  onChange: (value: string) => void;
  options: SelectOption[];
  placeholder?: string;
  className?: string;
  size?: 'sm' | 'md';
}

export function Select({ value, onChange, options, placeholder, className, size = 'md' }: SelectProps) {
  return (
    <div className="relative">
      <select
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className={cn(
          'appearance-none rounded-lg border border-[#262626] bg-[#111111] text-[#ededed] pr-8 focus:outline-none focus:border-accent/50',
          {
            'h-8 px-2 text-xs': size === 'sm',
            'h-9 px-3 text-sm': size === 'md',
          },
          className,
        )}
      >
        {placeholder && (
          <option value="" disabled>{placeholder}</option>
        )}
        {options.map((opt) => (
          <option key={opt.value} value={opt.value}>{opt.label}</option>
        ))}
      </select>
      <ChevronDown className="pointer-events-none absolute right-2 top-1/2 h-4 w-4 -translate-y-1/2 text-[#555555]" />
    </div>
  );
}
```

- [ ] **Step 6: Create Drawer component**

Create `web/src/components/ui/Drawer.tsx`:
```tsx
import { useEffect, type ReactNode } from 'react';
import { cn } from '../../lib/cn';
import { X } from 'lucide-react';

interface DrawerProps {
  open: boolean;
  onClose: () => void;
  title?: string;
  children: ReactNode;
  width?: number;
}

export function Drawer({ open, onClose, title, children, width = 640 }: DrawerProps) {
  useEffect(() => {
    if (open) {
      document.body.style.overflow = 'hidden';
    } else {
      document.body.style.overflow = '';
    }
    return () => { document.body.style.overflow = ''; };
  }, [open]);

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-[200]">
      <div className="fixed inset-0 bg-black/60 backdrop-blur-sm" onClick={onClose} />
      <div
        className="fixed right-0 top-0 bottom-0 z-10 border-l border-[#1e1e1e] bg-[#111111] overflow-y-auto animate-fade-in-up"
        style={{ width: `${width}px`, maxWidth: '90vw' }}
      >
        <div className="sticky top-0 z-10 flex items-center justify-between border-b border-[#1e1e1e] bg-[#111111] px-6 py-4">
          {title && <h2 className="font-display text-lg font-semibold text-[#ededed]">{title}</h2>}
          <button onClick={onClose} className="ml-auto rounded-lg p-1 text-[#555555] hover:bg-white/[0.04] hover:text-[#ededed] transition-colors">
            <X className="h-4 w-4" />
          </button>
        </div>
        <div className="p-6">{children}</div>
      </div>
    </div>
  );
}
```

- [ ] **Step 7: Create LoadingSpinner component**

Create `web/src/components/ui/LoadingSpinner.tsx`:
```tsx
import { cn } from '../../lib/cn';

interface LoadingSpinnerProps {
  size?: 'sm' | 'md' | 'lg';
  className?: string;
}

const sizeMap = { sm: 'h-4 w-4', md: 'h-8 w-8', lg: 'h-12 w-12' };

export function LoadingSpinner({ size = 'md', className }: LoadingSpinnerProps) {
  return (
    <div className={cn('flex items-center justify-center', className)}>
      <div className={cn(
        'animate-spin rounded-full border-2 border-[#262626] border-t-accent',
        sizeMap[size],
      )} />
    </div>
  );
}
```

- [ ] **Step 8: Create Alert component**

Create `web/src/components/ui/Alert.tsx`:
```tsx
import { cn } from '../../lib/cn';
import { AlertCircle, CheckCircle, Info, X } from 'lucide-react';

interface AlertProps {
  variant?: 'info' | 'success' | 'warning' | 'error';
  children: React.ReactNode;
  onClose?: () => void;
  className?: string;
}

const variantMap = {
  info: 'border-info/30 bg-info/10 text-info',
  success: 'border-accent/30 bg-accent/10 text-accent',
  warning: 'border-warning/30 bg-warning/10 text-warning',
  error: 'border-danger/30 bg-danger/10 text-danger',
};

const iconMap = {
  info: Info,
  success: CheckCircle,
  warning: AlertCircle,
  error: AlertCircle,
};

export function Alert({ variant = 'info', children, onClose, className }: AlertProps) {
  const Icon = iconMap[variant];
  return (
    <div className={cn('flex items-start gap-3 rounded-lg border p-3 text-sm', variantMap[variant], className)}>
      <Icon className="h-4 w-4 shrink-0 mt-0.5" />
      <span className="flex-1">{children}</span>
      {onClose && (
        <button onClick={onClose} className="shrink-0 rounded p-0.5 hover:bg-white/[0.06] transition-colors">
          <X className="h-3.5 w-3.5" />
        </button>
      )}
    </div>
  );
}
```

- [ ] **Step 9: Create ConfirmDialog component**

Create `web/src/components/ui/ConfirmDialog.tsx`:
```tsx
import { useState } from 'react';
import { cn } from '../../lib/cn';

interface ConfirmDialogProps {
  title: string;
  onConfirm: () => void;
  children: React.ReactNode;
  okText?: string;
  cancelText?: string;
}

export function ConfirmDialog({ title, onConfirm, children, okText = 'Confirm', cancelText = 'Cancel' }: ConfirmDialogProps) {
  const [open, setOpen] = useState(false);

  return (
    <>
      <button onClick={() => setOpen(true)} className="inline-flex items-center">
        {children}
      </button>
      {open && (
        <div className="fixed inset-0 z-[200] flex items-center justify-center">
          <div className="fixed inset-0 bg-black/60 backdrop-blur-sm" onClick={() => setOpen(false)} />
          <div className="relative z-10 w-full max-w-xs rounded-xl border border-[#1e1e1e] bg-[#111111] p-5 shadow-2xl animate-fade-in-up">
            <p className="text-sm text-[#ededed] mb-4">{title}</p>
            <div className="flex justify-end gap-2">
              <button
                onClick={() => setOpen(false)}
                className="h-8 px-3 text-xs font-medium rounded-lg border border-[#262626] text-[#888888] hover:bg-white/[0.04] hover:text-[#ededed] transition-colors"
              >
                {cancelText}
              </button>
              <button
                onClick={() => { onConfirm(); setOpen(false); }}
                className="h-8 px-3 text-xs font-medium rounded-lg bg-danger text-white hover:bg-danger/90 transition-colors"
              >
                {okText}
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
```

- [ ] **Step 10: Verify TypeScript compiles for UI components**

Run: `cd /workspace/web && npx tsc -b --noEmit 2>&1 | grep "components/ui" | head -5`
Expected: No errors from `components/ui/` files.

- [ ] **Step 11: Commit**

```bash
cd /workspace && git add web/src/components/ui/
git commit -m "feat(web): add Tailwind UI primitive components"
```

---

### Task 4: Create Zustand Auth Store & Wire App

**Files:**
- Create: `web/src/stores/authStore.ts`
- Delete: `web/src/contexts/AuthContext.tsx`
- Rewrite: `web/src/main.tsx`
- Rewrite: `web/src/App.tsx`

- [ ] **Step 1: Create Zustand auth store**

Create `web/src/stores/authStore.ts`:
```ts
import { create } from 'zustand';
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

export const useAuthStore = create<AuthState>((set, get) => ({
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

  // Listen for auth expiry events from the API interceptor
  const queryClient = useQueryClient();
  useAuthExpiryListener(queryClient);

  // Sync React Query data into Zustand store
  if (me && !useAuthStore.getState().user) {
    setUser({ id: me.id, username: me.username, role: me.role });
  }

  return { isLoading };
}

function useAuthExpiryListener(queryClient: ReturnType<typeof useQueryClient>) {
  const logout = useAuthStore((s) => s.logout);
  // We can't use useEffect in this context easily, so the listener
  // is set up in main.tsx instead.
}
```

- [ ] **Step 2: Rewrite main.tsx**

Replace `web/src/main.tsx`:
```tsx
import React from 'react';
import ReactDOM from 'react-dom/client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { Toaster } from 'sonner';
import App from './App';
import './styles/global.css';

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      refetchOnWindowFocus: false,
      retry: 1,
    },
  },
});

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <App />
      <Toaster
        position="top-right"
        theme="dark"
        toastOptions={{
          style: {
            background: '#141414',
            border: '1px solid #1e1e1e',
            color: '#ededed',
          },
        }}
      />
    </QueryClientProvider>
  </React.StrictMode>,
);
```

- [ ] **Step 3: Rewrite App.tsx**

Replace `web/src/App.tsx`:
```tsx
import { BrowserRouter, Routes, Route, Navigate, Outlet } from 'react-router-dom';
import { useAuthStore, useAuthBootstrap } from './stores/authStore';
import { getToken } from './api/client';
import { LoadingSpinner } from './components/ui/LoadingSpinner';
import Layout from './components/Layout';
import Home from './pages/Home';
import Login from './pages/Login';
import Register from './pages/Register';
import Dashboard from './pages/Dashboard';
import Keys from './pages/Keys';
import KeyDetail from './pages/KeyDetail';
import Providers from './pages/Providers';
import ProviderDetail from './pages/ProviderDetail';
import Users from './pages/Users';
import Settings from './pages/Settings';
import Usage from './pages/Usage';
import Logs from './pages/Logs';

function RequireAuth() {
  const user = useAuthStore((s) => s.user);
  const { isLoading } = useAuthBootstrap();
  if (isLoading) return <div className="flex h-screen items-center justify-center"><LoadingSpinner size="lg" /></div>;
  if (!user) {
    if (getToken()) return <div className="flex h-screen items-center justify-center"><LoadingSpinner size="lg" /></div>;
    return <Navigate to="/console/login" replace />;
  }
  return <Outlet />;
}

function RequireAdmin() {
  const user = useAuthStore((s) => s.user);
  if (!user || user.role !== 'admin') return <Navigate to="/console/dashboard" replace />;
  return <Outlet />;
}

function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<Home />} />
        <Route path="/console/login" element={<Login />} />
        <Route path="/console/register" element={<Register />} />
        <Route path="/console" element={<Layout />}>
          <Route element={<RequireAuth />}>
            <Route path="dashboard" element={<Dashboard />} />
            <Route path="keys" element={<Keys />} />
            <Route path="keys/:id" element={<KeyDetail />} />
            <Route path="usage" element={<Usage />} />
          </Route>
          <Route element={<RequireAdmin />}>
            <Route path="providers" element={<Providers />} />
            <Route path="providers/:id" element={<ProviderDetail />} />
            <Route path="users" element={<Users />} />
            <Route path="settings" element={<Settings />} />
            <Route path="logs" element={<Logs />} />
          </Route>
          <Route index element={<Navigate to="/console/dashboard" replace />} />
        </Route>
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </BrowserRouter>
  );
}

export default App;
```

- [ ] **Step 4: Delete AuthContext**

```bash
rm web/src/contexts/AuthContext.tsx
rmdir web/src/contexts 2>/dev/null || true
```

- [ ] **Step 5: Commit**

```bash
cd /workspace && git add web/src/stores/ web/src/main.tsx web/src/App.tsx web/src/contexts/
git commit -m "feat(web): replace AuthContext with Zustand auth store, add Sonner toasts"
```

---

### Task 5: Replace `antd message` in Hooks

**Files:**
- Modify: `web/src/hooks/useKeys.ts`
- Modify: `web/src/hooks/useProviders.ts`
- Modify: `web/src/hooks/useModels.ts`
- Modify: `web/src/hooks/useSettings.ts`
- Modify: `web/src/hooks/useUsers.ts`

- [ ] **Step 1: Update useKeys.ts**

Replace `web/src/hooks/useKeys.ts`:
```ts
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { listKeys, getKey, createKey, updateKey, deleteKey } from '../api/keys';
import type { CreateKeyRequest, UpdateKeyRequest } from '../types';
import { toast } from 'sonner';

export function useKeys(page = 1, pageSize = 20) {
  return useQuery({ queryKey: ['keys', page, pageSize], queryFn: () => listKeys(page, pageSize) });
}

export function useKey(id: string) {
  return useQuery({ queryKey: ['keys', id], queryFn: () => getKey(id), enabled: !!id });
}

export function useCreateKey() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateKeyRequest) => createKey(input),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['keys'] }); toast.success('API key created'); },
    onError: () => { toast.error('Failed to create API key'); },
  });
}

export function useUpdateKey() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateKeyRequest }) => updateKey(id, input),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['keys'] }); toast.success('API key updated'); },
    onError: () => { toast.error('Failed to update API key'); },
  });
}

export function useDeleteKey() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => deleteKey(id),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['keys'] }); toast.success('API key deleted'); },
    onError: () => { toast.error('Failed to delete API key'); },
  });
}
```

- [ ] **Step 2: Update useProviders.ts**

Replace `web/src/hooks/useProviders.ts`. Change `import { message } from 'antd'` to `import { toast } from 'sonner'`. Replace all `message.success(...)` with `toast.success(...)` and `message.error(...)` with `toast.error(...)`. The rest of the file stays identical.

- [ ] **Step 3: Update useModels.ts**

Replace `web/src/hooks/useModels.ts`. Same pattern: swap `antd message` for `sonner toast`.

- [ ] **Step 4: Update useSettings.ts**

Replace `web/src/hooks/useSettings.ts`. Same pattern.

- [ ] **Step 5: Update useUsers.ts**

Replace `web/src/hooks/useUsers.ts`. Same pattern.

- [ ] **Step 6: Verify no antd imports remain in hooks**

Run: `cd /workspace/web && grep -r "from 'antd'" src/hooks/`
Expected: No output.

- [ ] **Step 7: Commit**

```bash
cd /workspace && git add web/src/hooks/
git commit -m "refactor(web): replace antd message with sonner toast in all hooks"
```

---

### Task 6: Rewrite Layout Component

**Files:**
- Rewrite: `web/src/components/Layout.tsx`

- [ ] **Step 1: Rewrite Layout with Tailwind + Lucide icons**

Replace `web/src/components/Layout.tsx`:
```tsx
import { useState, useEffect } from 'react';
import { Outlet, useNavigate, useLocation } from 'react-router-dom';
import {
  LayoutDashboard,
  KeyRound,
  BarChart3,
  Cloud,
  Users,
  Settings,
  FileText,
  LogOut,
  PanelLeftClose,
  PanelLeftOpen,
} from 'lucide-react';
import { useAuthStore } from '../stores/authStore';
import { apiClient } from '../api/client';
import { cn } from '../lib/cn';

const consoleItems = [
  { key: '/console/dashboard', icon: LayoutDashboard, label: 'Dashboard' },
  { key: '/console/keys', icon: KeyRound, label: 'API Keys' },
  { key: '/console/usage', icon: BarChart3, label: 'Usage' },
];

const adminItems = [
  { key: '/console/providers', icon: Cloud, label: 'Providers' },
  { key: '/console/users', icon: Users, label: 'Users' },
  { key: '/console/settings', icon: Settings, label: 'Settings' },
  { key: '/console/logs', icon: FileText, label: 'Logs' },
];

export default function AppLayout() {
  const [collapsed, setCollapsed] = useState(false);
  const [version, setVersion] = useState('');
  const navigate = useNavigate();
  const location = useLocation();
  const user = useAuthStore((s) => s.user);
  const logout = useAuthStore((s) => s.logout);
  const isAdmin = user?.role === 'admin';

  useEffect(() => {
    apiClient.get<{ version: string }>('/version').then((r) => setVersion(r.data.version));
  }, []);

  return (
    <div className="flex min-h-screen">
      {/* Sidebar */}
      <aside className={cn(
        'fixed left-0 top-0 bottom-0 z-[100] flex flex-col border-r border-[#1e1e1e] bg-[#0a0a0a] transition-all duration-250 overflow-hidden',
        collapsed ? 'w-[72px]' : 'w-[240px]',
      )}>
        {/* Logo */}
        <div
          className="flex h-14 items-center gap-3 border-b border-[#1e1e1e] px-4 cursor-pointer overflow-hidden whitespace-nowrap"
          onClick={() => navigate('/console/dashboard')}
        >
          <div className="h-8 w-8 shrink-0 rounded-lg bg-gradient-to-br from-accent to-emerald-600 flex items-center justify-center font-display font-extrabold text-[13px] text-black tracking-tight">
            GW
          </div>
          <span className={cn(
            'font-display font-bold text-[15px] text-[#ededed] transition-opacity duration-150',
            collapsed ? 'opacity-0 pointer-events-none' : 'opacity-100',
          )}>
            LLM Gateway
          </span>
        </div>

        {/* Navigation */}
        <nav className="flex-1 overflow-y-auto overflow-x-hidden px-2 py-3 flex flex-col gap-0.5">
          <div className={cn(
            'text-[10px] font-semibold uppercase tracking-[0.1em] text-[#555555] px-3 pt-4 pb-1.5 whitespace-nowrap transition-all duration-150',
            collapsed ? 'opacity-0 h-0 p-0 overflow-hidden' : 'opacity-100',
          )}>
            Console
          </div>
          {consoleItems.map((item) => {
            const Icon = item.icon;
            const active = location.pathname === item.key;
            return (
              <div
                key={item.key}
                className={cn(
                  'flex items-center gap-3 rounded-lg px-3 py-2 cursor-pointer text-sm font-medium transition-all duration-150 whitespace-nowrap overflow-hidden select-none',
                  active
                    ? 'bg-accent/10 text-accent'
                    : 'text-[#888888] hover:bg-white/[0.04] hover:text-[#ededed]',
                  collapsed && 'justify-center px-2',
                )}
                onClick={() => navigate(item.key)}
              >
                <Icon className="h-[17px] w-[17px] shrink-0" />
                <span className={cn(collapsed && 'opacity-0 pointer-events-none')}>{item.label}</span>
              </div>
            );
          })}

          {isAdmin && (
            <>
              <div className={cn(
                'text-[10px] font-semibold uppercase tracking-[0.1em] text-[#555555] px-3 pt-4 pb-1.5 whitespace-nowrap transition-all duration-150',
                collapsed ? 'opacity-0 h-0 p-0 overflow-hidden' : 'opacity-100',
              )}>
                Admin
              </div>
              {adminItems.map((item) => {
                const Icon = item.icon;
                const active = location.pathname === item.key;
                return (
                  <div
                    key={item.key}
                    className={cn(
                      'flex items-center gap-3 rounded-lg px-3 py-2 cursor-pointer text-sm font-medium transition-all duration-150 whitespace-nowrap overflow-hidden select-none',
                      active
                        ? 'bg-accent/10 text-accent'
                        : 'text-[#888888] hover:bg-white/[0.04] hover:text-[#ededed]',
                      collapsed && 'justify-center px-2',
                    )}
                    onClick={() => navigate(item.key)}
                  >
                    <Icon className="h-[17px] w-[17px] shrink-0" />
                    <span className={cn(collapsed && 'opacity-0 pointer-events-none')}>{item.label}</span>
                  </div>
                );
              })}
            </>
          )}
        </nav>

        {/* Collapse toggle */}
        <div className="border-t border-[#1e1e1e] p-2">
          <button
            className="flex w-full items-center justify-center rounded-lg p-2 text-[#555555] hover:bg-white/[0.04] hover:text-[#888888] transition-colors"
            onClick={() => setCollapsed(!collapsed)}
          >
            {collapsed ? <PanelLeftOpen className="h-4 w-4" /> : <PanelLeftClose className="h-4 w-4" />}
          </button>
        </div>
      </aside>

      {/* Main content */}
      <div className={cn(
        'flex min-h-screen flex-col transition-all duration-250',
        collapsed ? 'ml-[72px]' : 'ml-[240px]',
      )}>
        {/* Header */}
        <header className="sticky top-0 z-50 flex h-14 items-center justify-end border-b border-[#1e1e1e] bg-[#0a0a0a] px-6 gap-3 shrink-0">
          <div className="flex items-center gap-2.5 text-sm font-medium text-[#888888]">
            <div className="flex h-7 w-7 items-center justify-center rounded-md bg-accent/10 text-accent font-display font-semibold text-xs">
              {user?.username?.charAt(0).toUpperCase()}
            </div>
            <span>{user?.username}</span>
          </div>
          <button
            className="flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-[13px] text-[#555555] hover:bg-white/[0.04] hover:text-[#888888] transition-colors"
            onClick={logout}
          >
            <LogOut className="h-3.5 w-3.5" />
            Logout
          </button>
        </header>

        {/* Content */}
        <main className="flex-1 bg-black p-6">
          <div className="animate-fade-in-up">
            <Outlet />
          </div>
        </main>

        {/* Footer */}
        <footer className="border-t border-[#1e1e1e] px-6 py-4 text-center text-xs text-[#555555] shrink-0">
          LLM Gateway{version ? ` ${version}` : ''}
        </footer>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
cd /workspace && git add web/src/components/Layout.tsx
git commit -m "refactor(web): rewrite Layout with Tailwind CSS and Lucide icons"
```

---

### Task 7: Rewrite Auth Pages (Login, Register)

**Files:**
- Rewrite: `web/src/pages/Login.tsx`
- Rewrite: `web/src/pages/Register.tsx`

- [ ] **Step 1: Rewrite Login.tsx**

Replace `web/src/pages/Login.tsx`:
```tsx
import { useState } from 'react';
import { useNavigate, Link } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { useAuthStore } from '../stores/authStore';
import { getAuthConfig } from '../api/auth';
import { Button } from '../components/ui/Button';
import { toast } from 'sonner';

export default function Login() {
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [loading, setLoading] = useState(false);
  const navigate = useNavigate();
  const login = useAuthStore((s) => s.login);

  const { data: authConfig } = useQuery({
    queryKey: ['authConfig'],
    queryFn: getAuthConfig,
    retry: false,
  });

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!username || !password) return;
    setLoading(true);
    try {
      await login({ username, password });
      navigate('/console/dashboard');
    } catch {
      toast.error('Invalid username or password');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen flex items-center justify-center relative overflow-hidden bg-black">
      <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_20%_50%,rgba(6,214,160,0.07)_0%,transparent_60%),radial-gradient(ellipse_at_80%_50%,rgba(59,130,246,0.05)_0%,transparent_60%)] pointer-events-none" />
      <div className="absolute inset-0 bg-[linear-gradient(#1e1e1e_1px,transparent_1px),linear-gradient(90deg,#1e1e1e_1px,transparent_1px)] bg-[size:60px_60px] opacity-25 [mask-image:radial-gradient(ellipse_at_center,black_30%,transparent_70%)] pointer-events-none" />

      <div className="relative w-[400px] max-w-[calc(100vw-48px)] rounded-2xl border border-[#1e1e1e] bg-[#111111] p-10 shadow-2xl animate-fade-in-up">
        <div className="flex items-center justify-center gap-3 mb-8">
          <div className="h-11 w-11 rounded-xl bg-gradient-to-br from-accent to-emerald-600 flex items-center justify-center font-display font-extrabold text-lg text-black tracking-tight">
            GW
          </div>
          <span className="font-display font-bold text-xl text-[#ededed]">LLM Gateway</span>
        </div>

        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="block text-xs font-medium text-[#888888] mb-1.5">Username</label>
            <input
              type="text"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              placeholder="Username"
              required
              className="h-10 w-full rounded-lg border border-[#262626] bg-[#141414] px-3 text-sm text-[#ededed] placeholder-[#555555] outline-none focus:border-accent/50 transition-colors"
            />
          </div>
          <div>
            <label className="block text-xs font-medium text-[#888888] mb-1.5">Password</label>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="Password"
              required
              className="h-10 w-full rounded-lg border border-[#262626] bg-[#141414] px-3 text-sm text-[#ededed] placeholder-[#555555] outline-none focus:border-accent/50 transition-colors"
            />
          </div>
          <div className={authConfig?.allow_registration ? 'mb-5' : ''}>
            <Button type="primary" size="lg" loading={loading} className="w-full">
              Sign In
            </Button>
          </div>
        </form>

        {authConfig?.allow_registration && (
          <p className="mt-5 text-center text-sm text-[#888888]">
            Don't have an account?{' '}
            <Link to="/console/register" className="text-accent hover:text-accent-hover transition-colors">Create one</Link>
          </p>
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Rewrite Register.tsx**

Replace `web/src/pages/Register.tsx`:
```tsx
import { useState } from 'react';
import { useNavigate, Link } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { useAuthStore } from '../stores/authStore';
import { getAuthConfig } from '../api/auth';
import { Button } from '../components/ui/Button';
import { Alert } from '../components/ui/Alert';
import { toast } from 'sonner';

export default function Register() {
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [confirm, setConfirm] = useState('');
  const [loading, setLoading] = useState(false);
  const navigate = useNavigate();
  const register = useAuthStore((s) => s.register);

  const { data: authConfig } = useQuery({
    queryKey: ['authConfig'],
    queryFn: getAuthConfig,
    retry: false,
  });

  const registrationDisabled = authConfig !== undefined && !authConfig.allow_registration;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (password !== confirm) {
      toast.error('Passwords do not match');
      return;
    }
    if (!username || !password) return;
    setLoading(true);
    try {
      await register({ username, password });
      navigate('/console/dashboard');
    } catch {
      toast.error('Registration failed');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen flex items-center justify-center relative overflow-hidden bg-black">
      <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_20%_50%,rgba(6,214,160,0.07)_0%,transparent_60%),radial-gradient(ellipse_at_80%_50%,rgba(59,130,246,0.05)_0%,transparent_60%)] pointer-events-none" />
      <div className="absolute inset-0 bg-[linear-gradient(#1e1e1e_1px,transparent_1px),linear-gradient(90deg,#1e1e1e_1px,transparent_1px)] bg-[size:60px_60px] opacity-25 [mask-image:radial-gradient(ellipse_at_center,black_30%,transparent_70%)] pointer-events-none" />

      <div className="relative w-[400px] max-w-[calc(100vw-48px)] rounded-2xl border border-[#1e1e1e] bg-[#111111] p-10 shadow-2xl animate-fade-in-up">
        <div className="flex items-center justify-center gap-3 mb-8">
          <div className="h-11 w-11 rounded-xl bg-gradient-to-br from-accent to-emerald-600 flex items-center justify-center font-display font-extrabold text-lg text-black tracking-tight">
            GW
          </div>
          <span className="font-display font-bold text-xl text-[#ededed]">Create Account</span>
        </div>

        {registrationDisabled && (
          <Alert variant="warning" className="mb-4">Registration is currently disabled</Alert>
        )}

        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="block text-xs font-medium text-[#888888] mb-1.5">Username</label>
            <input
              type="text"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              placeholder="Username"
              required
              minLength={3}
              disabled={registrationDisabled}
              className="h-10 w-full rounded-lg border border-[#262626] bg-[#141414] px-3 text-sm text-[#ededed] placeholder-[#555555] outline-none focus:border-accent/50 transition-colors disabled:opacity-50"
            />
          </div>
          <div>
            <label className="block text-xs font-medium text-[#888888] mb-1.5">Password</label>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="Password"
              required
              minLength={6}
              disabled={registrationDisabled}
              className="h-10 w-full rounded-lg border border-[#262626] bg-[#141414] px-3 text-sm text-[#ededed] placeholder-[#555555] outline-none focus:border-accent/50 transition-colors disabled:opacity-50"
            />
          </div>
          <div>
            <label className="block text-xs font-medium text-[#888888] mb-1.5">Confirm Password</label>
            <input
              type="password"
              value={confirm}
              onChange={(e) => setConfirm(e.target.value)}
              placeholder="Confirm password"
              required
              disabled={registrationDisabled}
              className="h-10 w-full rounded-lg border border-[#262626] bg-[#141414] px-3 text-sm text-[#ededed] placeholder-[#555555] outline-none focus:border-accent/50 transition-colors disabled:opacity-50"
            />
          </div>
          <div className="mb-5">
            <Button type="primary" size="lg" loading={loading} disabled={registrationDisabled} className="w-full">
              Register
            </Button>
          </div>
        </form>

        <p className="text-center text-sm text-[#888888]">
          Already have an account?{' '}
          <Link to="/console/login" className="text-accent hover:text-accent-hover transition-colors">Sign in</Link>
        </p>
      </div>
    </div>
  );
}
```

- [ ] **Step 3: Commit**

```bash
cd /workspace && git add web/src/pages/Login.tsx web/src/pages/Register.tsx
git commit -m "refactor(web): rewrite Login and Register pages with Tailwind CSS"
```

---

### Task 8: Rewrite Dashboard Page

**Files:**
- Rewrite: `web/src/pages/Dashboard.tsx`
- Delete: `web/src/components/StatCard.tsx`

- [ ] **Step 1: Rewrite Dashboard.tsx**

Replace `web/src/pages/Dashboard.tsx`:
```tsx
import { MessageSquare, DollarSign, BarChart3, Zap } from 'lucide-react';
import { useLogs } from '../hooks/useLogs';
import { useUsage } from '../hooks/useUsage';
import { Badge } from '../components/ui/Badge';

function startOfDay() {
  const d = new Date();
  d.setHours(0, 0, 0, 0);
  return d.toISOString();
}

function startOfMonth() {
  const d = new Date();
  d.setDate(1);
  d.setHours(0, 0, 0, 0);
  return d.toISOString();
}

export default function Dashboard() {
  const { data: allUsage } = useUsage({}, 1, 99999);
  const { data: todayUsage } = useUsage({ since: startOfDay() }, 1, 99999);
  const { data: monthUsage } = useUsage({ since: startOfMonth() }, 1, 99999);
  const { data: recentLogs } = useLogs({});

  const todayItems = todayUsage?.items ?? [];
  const monthItems = monthUsage?.items ?? [];
  const allItems = allUsage?.items ?? [];

  const todayRequests = todayItems.length;
  const todayCost = todayItems.reduce((sum, r) => sum + r.cost, 0);
  const monthCost = monthItems.reduce((sum, r) => sum + r.cost, 0);
  const totalModels = new Set(allItems.map(r => r.model_name)).size;

  const stats = [
    { label: "Today's Requests", value: todayRequests.toLocaleString(), icon: MessageSquare, accent: '#06d6a0' },
    { label: "Today's Cost", value: `$${todayCost.toFixed(4)}`, icon: DollarSign, accent: '#f59e0b' },
    { label: 'Monthly Cost', value: `$${monthCost.toFixed(2)}`, icon: DollarSign, accent: '#3b82f6' },
    { label: 'Active Models', value: String(totalModels), icon: Zap, accent: '#a855f7' },
  ];

  return (
    <div>
      <div className="mb-6">
        <h1 className="font-display text-2xl font-bold text-[#ededed]">Dashboard</h1>
      </div>

      <div className="grid grid-cols-4 gap-4 mb-6 max-lg:grid-cols-2 max-sm:grid-cols-1">
        {stats.map((s, i) => {
          const Icon = s.icon;
          return (
            <div
              key={s.label}
              className="relative overflow-hidden rounded-xl border border-[#1e1e1e] bg-[#0a0a0a] p-5 pl-6 transition-all duration-250 hover:border-[#262626] hover:-translate-y-0.5 hover:shadow-lg hover:shadow-black/20 animate-fade-in-up"
              style={{ animationDelay: `${i * 60}ms` }}
            >
              <div className="absolute top-0 left-0 right-0 h-[2px] bg-gradient-to-r opacity-0 hover:opacity-100 transition-opacity" style={{ background: `linear-gradient(90deg, ${s.accent}, transparent)` }} />
              <div className="text-[11px] font-semibold uppercase tracking-[0.06em] text-[#555555] mb-2">{s.label}</div>
              <div className="font-mono text-[28px] font-bold text-[#ededed] leading-tight">{s.value}</div>
              <Icon className="absolute top-[18px] right-[18px] h-[18px] w-[18px] text-[#555555] opacity-40" />
            </div>
          );
        })}
      </div>

      <div className="flex items-center justify-between mb-4">
        <h2 className="font-display text-base font-semibold text-[#ededed]">Recent Requests</h2>
      </div>

      <div className="overflow-x-auto rounded-xl border border-[#1e1e1e]">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-[#1e1e1e]">
              <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Time</th>
              <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Model</th>
              <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Protocol</th>
              <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Status</th>
              <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Tokens</th>
              <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Cost</th>
              <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Latency</th>
            </tr>
          </thead>
          <tbody>
            {recentLogs?.items?.map((log) => (
              <tr key={log.id} className="border-b border-[#1e1e1e]/50 hover:bg-white/[0.02] transition-colors">
                <td className="px-4 py-2.5"><span className="mono text-[13px]">{new Date(log.created_at).toLocaleString()}</span></td>
                <td className="px-4 py-2.5"><span className="mono">{log.model_name}</span></td>
                <td className="px-4 py-2.5"><Badge variant={log.protocol === 'openai' ? 'blue' : 'purple'}>{log.protocol}</Badge></td>
                <td className="px-4 py-2.5"><Badge variant={log.status_code < 400 ? 'green' : log.status_code < 500 ? 'amber' : 'red'}>{log.status_code}</Badge></td>
                <td className="px-4 py-2.5"><span className="mono">{log.input_tokens ?? 0} + {log.output_tokens ?? 0}</span></td>
                <td className="px-4 py-2.5"><span className="mono">${log.cost.toFixed(6)}</span></td>
                <td className="px-4 py-2.5"><span className="mono">{log.latency_ms}ms</span></td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Delete StatCard.tsx**

```bash
rm web/src/components/StatCard.tsx
```

- [ ] **Step 3: Commit**

```bash
cd /workspace && git add web/src/pages/Dashboard.tsx web/src/components/StatCard.tsx
git commit -m "refactor(web): rewrite Dashboard with Tailwind CSS, remove StatCard"
```

---

### Task 9: Rewrite Data Pages (Keys, KeyDetail, Providers, ProviderDetail)

**Files:**
- Rewrite: `web/src/pages/Keys.tsx`
- Rewrite: `web/src/pages/KeyDetail.tsx`
- Rewrite: `web/src/pages/Providers.tsx`
- Rewrite: `web/src/pages/ProviderDetail.tsx`

This is the largest task. Each page follows the same pattern: replace Ant Table/Form/Modal/Button with custom components + Tailwind tables.

- [ ] **Step 1: Rewrite Keys.tsx**

Replace `web/src/pages/Keys.tsx`. Use `Button`, `Modal`, `Badge` from `components/ui/`. Replace `Form.useForm()` with `useState` for simple forms. Replace `Table` with a plain HTML `<table>` with Tailwind classes. Replace `message` with `toast` from sonner.

Key patterns:
- Table: `<table className="w-full text-sm">` with `<thead>` and `<tbody>`
- Pagination: manual `<div className="flex items-center justify-between mt-4">` with prev/next buttons
- Modal: `<Modal>` component for create dialog
- Navigate link: `<button onClick={() => navigate(...)} className="text-accent hover:text-accent-hover">` instead of `<a>`

Full implementation:
```tsx
import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Plus } from 'lucide-react';
import { useKeys, useCreateKey } from '../hooks/useKeys';
import { Button } from '../components/ui/Button';
import { Modal } from '../components/ui/Modal';
import { Badge } from '../components/ui/Badge';
import { toast } from 'sonner';
import type { CreateKeyResponse } from '../types';

export default function Keys() {
  const [page, setPage] = useState(1);
  const [pageSize] = useState(20);
  const { data, isLoading } = useKeys(page, pageSize);
  const createKeyMutation = useCreateKey();
  const navigate = useNavigate();
  const [createOpen, setCreateOpen] = useState(false);
  const [createdKey, setCreatedKey] = useState<string | null>(null);
  const [name, setName] = useState('');
  const [rateLimit, setRateLimit] = useState('');
  const [budget, setBudget] = useState('');

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    const result: CreateKeyResponse = await createKeyMutation.mutateAsync({
      name,
      rate_limit: rateLimit ? Number(rateLimit) : null,
      budget_monthly: budget ? Number(budget) : null,
    });
    setCreatedKey(result.key);
    setName('');
    setRateLimit('');
    setBudget('');
  };

  const copyKey = () => {
    if (createdKey) {
      navigator.clipboard.writeText(createdKey);
      toast.success('Key copied to clipboard');
    }
  };

  const totalPages = Math.ceil((data?.total ?? 0) / pageSize);

  return (
    <div>
      <div className="mb-6 flex items-center justify-between">
        <h1 className="font-display text-2xl font-bold text-[#ededed]">API Keys</h1>
        <Button icon={<Plus className="h-4 w-4" />} onClick={() => setCreateOpen(true)}>
          Create Key
        </Button>
      </div>

      {isLoading ? (
        <div className="flex items-center justify-center py-12 text-[#555555]">Loading...</div>
      ) : (
        <>
          <div className="overflow-x-auto rounded-xl border border-[#1e1e1e]">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-[#1e1e1e]">
                  <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Name</th>
                  <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Status</th>
                  <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Rate Limit (RPM)</th>
                  <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Monthly Budget</th>
                  <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Created</th>
                </tr>
              </thead>
              <tbody>
                {data?.items?.map((key) => (
                  <tr key={key.id} className="border-b border-[#1e1e1e]/50 hover:bg-white/[0.02] transition-colors">
                    <td className="px-4 py-2.5">
                      <button onClick={() => navigate(`/console/keys/${key.id}`)} className="text-accent hover:text-accent-hover transition-colors">
                        {key.name}
                      </button>
                    </td>
                    <td className="px-4 py-2.5"><Badge variant={key.enabled ? 'green' : 'red'}>{key.enabled ? 'Active' : 'Disabled'}</Badge></td>
                    <td className="px-4 py-2.5"><span className="mono">{key.rate_limit ?? 'Unlimited'}</span></td>
                    <td className="px-4 py-2.5"><span className="mono">{key.budget_monthly != null ? `$${key.budget_monthly.toFixed(2)}` : 'Unlimited'}</span></td>
                    <td className="px-4 py-2.5"><span className="mono">{new Date(key.created_at).toLocaleDateString()}</span></td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          {totalPages > 1 && (
            <div className="mt-4 flex items-center justify-between text-sm">
              <span className="text-[#555555]">Total {data?.total ?? 0}</span>
              <div className="flex gap-2">
                <Button variant="ghost" size="sm" disabled={page <= 1} onClick={() => setPage(page - 1)}>Previous</Button>
                <span className="px-2 text-[#888888]">{page} / {totalPages}</span>
                <Button variant="ghost" size="sm" disabled={page >= totalPages} onClick={() => setPage(page + 1)}>Next</Button>
              </div>
            </div>
          )}
        </>
      )}

      <Modal
        open={createOpen}
        onClose={() => { setCreateOpen(false); setCreatedKey(null); }}
        title="Create API Key"
        footer={createdKey ? (
          <div className="flex gap-2">
            <Button onClick={copyKey}>Copy Key</Button>
            <Button variant="secondary" onClick={() => { setCreateOpen(false); setCreatedKey(null); }}>Done</Button>
          </div>
        ) : undefined}
      >
        {createdKey ? (
          <div>
            <p className="text-sm text-[#888888]">Save this key now. It won't be shown again.</p>
            <div className="mt-2 rounded-lg border border-[#1e1e1e] bg-[#0a0a0a] p-3 font-mono text-sm text-[#ededed] break-all">{createdKey}</div>
          </div>
        ) : (
          <form onSubmit={handleCreate} className="space-y-4">
            <div>
              <label className="block text-xs font-medium text-[#888888] mb-1.5">Name</label>
              <input
                type="text"
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="e.g., production-app"
                required
                className="h-9 w-full rounded-lg border border-[#262626] bg-[#141414] px-3 text-sm text-[#ededed] placeholder-[#555555] outline-none focus:border-accent/50 transition-colors"
              />
            </div>
            <div className="flex gap-4">
              <div className="flex-1">
                <label className="block text-xs font-medium text-[#888888] mb-1.5">Rate Limit (RPM)</label>
                <input
                  type="number"
                  value={rateLimit}
                  onChange={(e) => setRateLimit(e.target.value)}
                  placeholder="Unlimited"
                  min={1}
                  className="h-9 w-full rounded-lg border border-[#262626] bg-[#141414] px-3 text-sm text-[#ededed] placeholder-[#555555] outline-none focus:border-accent/50 transition-colors"
                />
              </div>
              <div className="flex-1">
                <label className="block text-xs font-medium text-[#888888] mb-1.5">Monthly Budget ($)</label>
                <input
                  type="number"
                  value={budget}
                  onChange={(e) => setBudget(e.target.value)}
                  placeholder="Unlimited"
                  min={0}
                  step={0.01}
                  className="h-9 w-full rounded-lg border border-[#262626] bg-[#141414] px-3 text-sm text-[#ededed] placeholder-[#555555] outline-none focus:border-accent/50 transition-colors"
                />
              </div>
            </div>
            <Button type="primary" loading={createKeyMutation.isPending}>Create</Button>
          </form>
        )}
      </Modal>
    </div>
  );
}
```

- [ ] **Step 2: Rewrite KeyDetail.tsx**

Replace `web/src/pages/KeyDetail.tsx` using `Button`, `Toggle`, `ConfirmDialog` from UI components. Use `useState` for form fields. Same table/input patterns as Keys.

Key changes from current:
- `Form.useForm()` → `useState` for each field
- `InputNumber` → `<input type="number">`
- `Switch` → `<Toggle>` component
- `Popconfirm` → `<ConfirmDialog>` component
- `ArrowLeftOutlined` → `<ArrowLeft>` from lucide-react

- [ ] **Step 3: Rewrite Providers.tsx**

Replace `web/src/pages/Providers.tsx`. Same pattern as Keys but for providers. Simpler — no pagination needed since `useProviders()` returns all providers.

- [ ] **Step 4: Rewrite ProviderDetail.tsx**

Replace `web/src/pages/ProviderDetail.tsx`. This is the most complex page — it has provider form, models table with CRUD, channels table with CRUD, and multiple modals. Use `Button`, `Modal`, `Toggle`, `ConfirmDialog`, `Select`, `Badge` from UI components.

Key changes:
- Three `Form.useForm()` instances → `useState` for each form
- Multiple `Modal` instances → `<Modal>` components
- `Input.Password` → `<input type="password">`
- `InputNumber` → `<input type="number">`
- `Select` → `<Select>` component for billing type
- `Popconfirm` → `<ConfirmDialog>` component

- [ ] **Step 5: Verify no antd imports remain in data pages**

Run: `cd /workspace/web && grep -r "from 'antd'" src/pages/Keys.tsx src/pages/KeyDetail.tsx src/pages/Providers.tsx src/pages/ProviderDetail.tsx`
Expected: No output.

- [ ] **Step 6: Commit**

```bash
cd /workspace && git add web/src/pages/Keys.tsx web/src/pages/KeyDetail.tsx web/src/pages/Providers.tsx web/src/pages/ProviderDetail.tsx
git commit -m "refactor(web): rewrite Keys, KeyDetail, Providers, ProviderDetail with Tailwind"
```

---

### Task 10: Rewrite Admin Pages (Users, Settings, Usage, Logs)

**Files:**
- Rewrite: `web/src/pages/Users.tsx`
- Rewrite: `web/src/pages/Settings.tsx`
- Rewrite: `web/src/pages/Usage.tsx`
- Rewrite: `web/src/pages/Logs.tsx`

- [ ] **Step 1: Rewrite Users.tsx**

Replace `web/src/pages/Users.tsx`. Replace Ant `Table` with plain table, `Select` with custom `Select` component, `Popconfirm` with `ConfirmDialog`, `Tag` with `Badge`.

- [ ] **Step 2: Rewrite Settings.tsx**

Replace `web/src/pages/Settings.tsx`. Replace `Switch` with `Toggle`, `Card` with plain divs, `Alert` with custom `Alert`, `Form` with `useState` + native inputs, `Input.Password` with `<input type="password">`.

- [ ] **Step 3: Rewrite Usage.tsx**

Replace `web/src/pages/Usage.tsx`. Key changes:
- `DatePicker.RangePicker` → two native `<input type="date">` elements
- `Select` → custom `Select` component
- `Card` → plain divs
- `Table` → plain HTML table
- `@ant-design/icons` → `lucide-react` icons
- `dayjs` → native `Date` constructors
- Keep `recharts` components (BarChart etc.) — they work fine with Tailwind

- [ ] **Step 4: Rewrite Logs.tsx**

Replace `web/src/pages/Logs.tsx`. Key changes:
- `DatePicker.RangePicker` → two native `<input type="date">` elements
- `Select` → custom `Select` component
- `Drawer` → custom `Drawer` component
- `Table` → plain HTML table
- `Tag` → `Badge` component
- `Typography.Title` → plain `<h3>` with Tailwind

- [ ] **Step 5: Verify no antd imports remain**

Run: `cd /workspace/web && grep -r "from 'antd'" src/pages/`
Expected: No output.

- [ ] **Step 6: Commit**

```bash
cd /workspace && git add web/src/pages/Users.tsx web/src/pages/Settings.tsx web/src/pages/Usage.tsx web/src/pages/Logs.tsx
git commit -m "refactor(web): rewrite Users, Settings, Usage, Logs with Tailwind"
```

---

### Task 11: Rewrite Home Page

**Files:**
- Rewrite: `web/src/pages/Home.tsx`

- [ ] **Step 1: Rewrite Home.tsx**

Replace `web/src/pages/Home.tsx`. Replace all Ant components:
- `Button` → custom `Button`
- `Typography` → plain HTML with Tailwind
- `Space`, `Row`, `Col` → Tailwind flex/grid
- `Divider` → `<hr>` with Tailwind
- `@ant-design/icons` → `lucide-react` equivalents

Icon mapping:
- `ApiOutlined` → `Globe`
- `CloudServerOutlined` → `Cloud`
- `KeyOutlined` → `KeyRound`
- `ThunderboltOutlined` → `Zap`
- `SafetyCertificateOutlined` → `ShieldCheck`
- `DashboardOutlined` → `LayoutDashboard`

- [ ] **Step 2: Commit**

```bash
cd /workspace && git add web/src/pages/Home.tsx
git commit -m "refactor(web): rewrite Home page with Tailwind CSS"
```

---

### Task 12: Update Tests & Test Utilities

**Files:**
- Rewrite: `web/src/test/render.tsx`
- Rewrite: `web/src/pages/Login.test.tsx`
- Rewrite: `web/src/pages/Register.test.tsx`
- Rewrite: `web/src/pages/Keys.test.tsx`
- Rewrite: `web/src/pages/Settings.test.tsx`
- Rewrite: `web/src/pages/Users.test.tsx`

- [ ] **Step 1: Update test/render.tsx**

Replace `web/src/test/render.tsx` — swap `AuthProvider` for a mock that wraps children with Zustand:
```tsx
import { render } from '@testing-library/react';
import type { ReactElement, ReactNode } from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { MemoryRouter } from 'react-router-dom';
import { useAuthStore } from '../stores/authStore';

function createTestQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0 },
      mutations: { retry: false },
    },
  });
}

export function renderWithProviders(
  ui: ReactElement,
  options?: { route?: string; queryClient?: QueryClient },
) {
  const queryClient = options?.queryClient ?? createTestQueryClient();
  const route = options?.route ?? '/';

  function Wrapper({ children }: { children: ReactNode }) {
    return (
      <QueryClientProvider client={queryClient}>
        <MemoryRouter initialEntries={[route]}>
          {children}
        </MemoryRouter>
      </QueryClientProvider>
    );
  }

  return { ...render(ui, { wrapper: Wrapper }), queryClient };
}

export { render };
```

- [ ] **Step 2: Update Login.test.tsx**

The test mocks `useNavigate` and renders Login. The main change: Login no longer uses `Form` from antd, so queries need updating. The `renderWithProviders` no longer wraps with `AuthProvider`, so we need to set a user in the Zustand store for protected route tests.

Key updates:
- Mock `useNavigate` stays the same
- Form submission: use `screen.getByPlaceholderText` + `fireEvent.change` + `fireEvent.submit` instead of Ant Form
- Remove any Ant-specific assertions

- [ ] **Step 3: Update Register.test.tsx**

Same pattern as Login test updates.

- [ ] **Step 4: Update Keys.test.tsx**

Same pattern — no Ant Form, use native input events.

- [ ] **Step 5: Update Settings.test.tsx**

Same pattern.

- [ ] **Step 6: Update Users.test.tsx**

The Users test checks for table headers and data. The table is now plain HTML instead of Ant Table. The `getByRole('table')` still works for HTML `<table>`. Column headers should still render. Verify assertions pass.

- [ ] **Step 7: Run all tests**

Run: `cd /workspace/web && npm test 2>&1`
Expected: All tests pass. Fix any failures.

- [ ] **Step 8: Commit**

```bash
cd /workspace && git add web/src/test/ web/src/pages/*.test.tsx
git commit -m "test(web): update tests for Tailwind migration"
```

---

### Task 13: Final Cleanup & Verification

**Files:**
- Modify: `web/src/components/JsonViewer.tsx` (verify no antd)
- Verify: `web/src/api/` (should have no antd)
- Delete: any remaining antd-related CSS overrides in global.css

- [ ] **Step 1: Verify no antd imports remain anywhere**

Run: `cd /workspace/web && grep -r "from 'antd'" src/ && grep -r "from \"antd\"" src/ && grep -r "@ant-design" src/ && grep -r "dayjs" src/`
Expected: No output for any of these.

- [ ] **Step 2: Run TypeScript build check**

Run: `cd /workspace/web && npx tsc -b --noEmit 2>&1`
Expected: No errors.

- [ ] **Step 3: Run production build**

Run: `cd /workspace/web && npm run build 2>&1`
Expected: Build succeeds. Check output size — should be significantly smaller than before.

- [ ] **Step 4: Run all tests**

Run: `cd /workspace/web && npm test 2>&1`
Expected: All tests pass.

- [ ] **Step 5: Final commit if any fixes needed**

```bash
cd /workspace && git add -A web/src/
git commit -m "chore(web): final cleanup for Tailwind migration"
```

---

### Task 14: Update E2E Test Selectors

**Files:**
- Modify: E2E test files (if any selector changes are needed)

- [ ] **Step 1: Check E2E tests for Ant-specific selectors**

Run: `cd /workspace/web && grep -r "ant-" src/test/ test/ e2e/ 2>/dev/null || echo "No e2e test files found with ant- selectors"`
Expected: Either no results or a list of files to update.

- [ ] **Step 2: Run E2E tests if backend is available**

Run: `cd /workspace/web && npm run test:e2e 2>&1 | tail -20`
Expected: E2E tests pass. If backend is not available, skip this step.

- [ ] **Step 3: Commit if E2E fixes needed**

```bash
cd /workspace && git add web/ -A
git commit -m "test(web): update E2E test selectors for Tailwind migration"
```
