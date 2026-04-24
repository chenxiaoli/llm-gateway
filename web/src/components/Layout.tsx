import { useState, useEffect } from 'react';
import { Outlet, useNavigate, useLocation } from 'react-router-dom';
import {
  LayoutDashboard,
  KeyRound,
  BarChart3,
  Globe,
  Cpu,
  Users,
  Settings,
  FileText,
  DollarSign,
  LogOut,
  PanelLeftClose,
  PanelLeftOpen,
  Sun,
  Moon,
  ChevronRight,
} from 'lucide-react';
import { useAuthStore } from '../stores/authStore';
import { useTheme } from '../hooks/useTheme';
import { apiClient } from '../api/client';

const consoleItems = [
  { key: '/console/dashboard', icon: LayoutDashboard, label: 'Dashboard' },
  { key: '/console/keys', icon: KeyRound, label: 'API Keys' },
  { key: '/console/usage', icon: BarChart3, label: 'Usage' },
];

const adminItems = [
  { key: '/console/channels', icon: Globe, label: 'Channels' },
  { key: '/console/providers', icon: Globe, label: 'Providers' },
  { key: '/console/models', icon: Cpu, label: 'Models' },
  { key: '/console/pricing-policies', icon: DollarSign, label: 'Pricing Policies' },
  { key: '/console/users', icon: Users, label: 'Users' },
  { key: '/console/settings', icon: Settings, label: 'Settings' },
  { key: '/console/logs', icon: FileText, label: 'Logs' },
];

// Map paths to display names for breadcrumbs
const routeLabels: Record<string, string> = {
  dashboard: 'Dashboard',
  keys: 'API Keys',
  usage: 'Usage',
  providers: 'Providers',
  channels: 'Channels',
  models: 'Models',
  'pricing-policies': 'Pricing Policies',
  users: 'Users',
  settings: 'Settings',
  logs: 'Audit Logs',
};

export default function AppLayout() {
  const [collapsed, setCollapsed] = useState(false);
  const [version, setVersion] = useState('');
  const navigate = useNavigate();
  const location = useLocation();
  const user = useAuthStore((s) => s.user);
  const logout = useAuthStore((s) => s.logout);
  const isAdmin = user?.role === 'admin';
  const { theme, toggleTheme } = useTheme();

  useEffect(() => {
    apiClient.get<{ version: string }>('/version').then((r) => setVersion(r.data.version));
  }, []);

  const sidebarWidth = collapsed ? 'w-[68px]' : 'w-[232px]';

  const breadcrumbSegment = location.pathname.replace('/console/', '');

  return (
    <div className="flex min-h-screen bg-base-200">
      {/* ── Sidebar ── */}
      <aside className={`fixed left-0 top-0 bottom-0 z-[100] flex flex-col bg-base-100 border-r border-base-300/60 transition-all duration-300 overflow-hidden ${sidebarWidth}`}>
        {/* Logo */}
        <div
          className="flex h-14 items-center gap-3 border-b border-base-300/60 px-4 cursor-pointer overflow-hidden whitespace-nowrap"
          onClick={() => navigate('/')}
        >
          <div className="h-8 w-8 shrink-0 rounded-lg bg-primary flex items-center justify-center font-semibold text-md text-primary-content tracking-tight">
            GW
          </div>
          <span className={`font-semibold text-lg transition-opacity duration-200 ${collapsed ? 'opacity-0' : 'opacity-100'}`}>
            LLM Gateway
          </span>
        </div>

        {/* Navigation */}
        <nav className="flex-1 overflow-y-auto overflow-x-hidden px-3 py-4 flex flex-col gap-0.5">
          {!collapsed && (
            <div className="text-xs font-semibold uppercase tracking-[0.12em] text-base-content/30 px-3 pt-1 pb-2">
              Console
            </div>
          )}
          {consoleItems.map((item) => {
            const Icon = item.icon;
            const active = location.pathname === item.key;
            return (
              <div
                key={item.key}
                className={`flex items-center gap-3 rounded-lg px-3 py-2 cursor-pointer text-lg font-medium transition-all duration-150 whitespace-nowrap overflow-hidden select-none relative ${
                  active
                    ? 'bg-primary/10 text-primary'
                    : 'text-base-content/50 hover:bg-base-200 hover:text-base-content/80'
                } ${collapsed ? 'justify-center px-2' : ''}`}
                onClick={() => navigate(item.key)}
              >
                {/* Active indicator bar */}
                {active && (
                  <div className="absolute left-0 top-1/2 -translate-y-1/2 w-[3px] h-4 rounded-r-full bg-primary" />
                )}
                <Icon className="h-[17px] w-[17px] shrink-0" strokeWidth={active ? 2 : 1.5} />
                <span className={collapsed ? 'opacity-0 pointer-events-none' : ''}>{item.label}</span>
              </div>
            );
          })}

          {isAdmin && (
            <>
              {!collapsed && (
                <div className="text-xs font-semibold uppercase tracking-[0.12em] text-base-content/30 px-3 pt-5 pb-2">
                  Admin
                </div>
              )}
              {adminItems.map((item) => {
                const Icon = item.icon;
                const active = location.pathname === item.key || location.pathname.startsWith(item.key + '/');
                return (
                  <div
                    key={item.key}
                    className={`flex items-center gap-3 rounded-lg px-3 py-2 cursor-pointer text-lg font-medium transition-all duration-150 whitespace-nowrap overflow-hidden select-none relative ${
                      active
                        ? 'bg-primary/10 text-primary'
                        : 'text-base-content/50 hover:bg-base-200 hover:text-base-content/80'
                    } ${collapsed ? 'justify-center px-2' : ''}`}
                    onClick={() => navigate(item.key)}
                  >
                    {active && (
                      <div className="absolute left-0 top-1/2 -translate-y-1/2 w-[3px] h-4 rounded-r-full bg-primary" />
                    )}
                    <Icon className="h-[17px] w-[17px] shrink-0" strokeWidth={active ? 2 : 1.5} />
                    <span className={collapsed ? 'opacity-0 pointer-events-none' : ''}>{item.label}</span>
                  </div>
                );
              })}
            </>
          )}
        </nav>

        {/* Footer */}
        <div className="border-t border-base-300/60 shrink-0">
          <div className="p-2 flex flex-col gap-1">
            {!collapsed && version && (
              <div className="flex items-center justify-between px-3 py-1.5">
                <span className="mono text-xs text-base-content/20 truncate">v{version}</span>
                <a
                  href="https://github.com/chenxiaoli/llm-gateway"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-base-content/15 hover:text-base-content/40 transition-colors"
                  aria-label="GitHub"
                >
                  <svg className="h-3.5 w-3.5" viewBox="0 0 16 16" fill="currentColor"><path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"/></svg>
                </a>
              </div>
            )}
            <button
              className={`btn btn-ghost btn-sm w-full ${collapsed ? 'btn-square justify-center' : 'gap-2 justify-start'}`}
              onClick={() => setCollapsed(!collapsed)}
              aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
            >
              {collapsed ? <PanelLeftOpen className="h-4 w-4" /> : (
                <>
                  <PanelLeftClose className="h-4 w-4 text-base-content/30" />
                  <span className="text-lg text-base-content/30">Collapse</span>
                </>
              )}
            </button>
          </div>
        </div>
      </aside>

      {/* ── Main content ── */}
      <div className={`flex min-h-screen flex-col transition-all duration-300 ${collapsed ? 'ml-[68px]' : 'ml-[232px]'}`}>
        {/* Header */}
        <header className={`fixed top-0 z-40 shrink-0 bg-base-100/80 backdrop-blur-md border-b border-base-300/60 h-12 ${collapsed ? 'left-[68px]' : 'left-[232px]'} right-0`}>
          <div className="flex h-12 items-center justify-between px-6 gap-3 w-full">
            {/* Left: Breadcrumb */}
            <nav className="flex items-center gap-1.5 text-xs min-w-0">
              <button
                onClick={() => navigate('/console/dashboard')}
                className="text-base-content/30 hover:text-base-content/50 transition-colors shrink-0 font-medium"
              >
                Console
              </button>
              {breadcrumbSegment !== 'dashboard' && (
                <>
                  <ChevronRight className="h-3 w-3 text-base-content/15 shrink-0" />
                  <span className="text-base-content/60 truncate">
                    {routeLabels[breadcrumbSegment] || breadcrumbSegment}
                  </span>
                </>
              )}
            </nav>

            {/* Right: Controls */}
            <div className="flex items-center gap-1">
              <button
                className="btn btn-ghost btn-sm btn-circle"
                onClick={toggleTheme}
                aria-label="Toggle theme"
              >
                {theme === 'dark' ? <Sun className="h-4 w-4" /> : <Moon className="h-4 w-4" />}
              </button>

              <div className="h-4 w-px bg-base-300/60 mx-1" />

              <div className="flex items-center gap-2 text-sm">
                <div className="avatar placeholder">
                  <div className="bg-primary/15 text-primary w-7 rounded-md flex items-center justify-center">
                    <span className="text-xs font-semibold">{user?.username?.charAt(0).toUpperCase()}</span>
                  </div>
                </div>
                <span className="hidden sm:inline text-[13px] font-medium text-base-content/60">{user?.username}</span>
              </div>

              <div className="h-4 w-px bg-base-300/60 mx-1" />

              <button
                className="btn btn-ghost btn-sm btn-circle"
                onClick={logout}
                aria-label="Logout"
                title="Logout"
              >
                <LogOut className="h-3.5 w-3.5 text-base-content/30" />
              </button>
            </div>
          </div>
        </header>

        {/* Content */}
        <main className="flex-1 p-6 overflow-y-auto pt-12 pb-8">
          <div className="animate-fade-in-up" key={location.pathname}>
            <Outlet />
          </div>
        </main>

        {/* Footer */}
        <footer className={`fixed bottom-0 z-40 shrink-0 border-t border-base-300/60 bg-base-100/50 h-8 ${collapsed ? 'left-[68px]' : 'left-[232px]'} right-0`}>
          <div className="flex items-center justify-between px-6 py-2 w-full h-full">
            <div className="flex items-center gap-3">
              {import.meta.env.VITE_DEV_MODE ? (
                <>
                  <span className="text-[11px] text-base-content/60 font-mono">dev</span>
                  <span className="text-[11px] text-base-content/40 font-mono">•</span>
                  <span className="text-[11px] text-base-content/40 font-mono">{import.meta.env.VITE_COMMIT_SHA?.slice(0, 7) || 'local'}</span>
                </>
              ) : (
                <>
                  <span className="text-[11px] text-base-content/60 font-mono">
                    LLM Gateway{version ? ` v${version}` : ''}
                  </span>
                  <span className="text-[11px] text-base-content/40 font-mono">•</span>
                  <span className="text-[11px] text-base-content/40 font-mono">{import.meta.env.VITE_COMMIT_SHA?.slice(0, 7) || ''}</span>
                </>
              )}
            </div>
            <span className="text-[11px] text-base-content/40 font-mono hidden sm:inline">
              {user?.role === 'admin' ? 'admin' : 'user'}
            </span>
          </div>
        </footer>
      </div>
    </div>
  );
}
