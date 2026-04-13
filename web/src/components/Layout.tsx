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
  Sun,
  Moon,
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
  const { theme, toggleTheme } = useTheme();

  useEffect(() => {
    apiClient.get<{ version: string }>('/version').then((r) => setVersion(r.data.version));
  }, []);

  const sidebarWidth = collapsed ? 'w-[72px]' : 'w-[240px]';

  return (
    <div className="flex min-h-screen bg-base-100">
      {/* Sidebar */}
      <aside className={`fixed left-0 top-0 bottom-0 z-[100] flex flex-col border-r border-base-300 bg-base-200 transition-all duration-300 overflow-hidden ${sidebarWidth}`}>
        {/* Logo */}
        <div
          className="flex h-14 items-center gap-3 border-b border-base-300 px-4 cursor-pointer overflow-hidden whitespace-nowrap"
          onClick={() => navigate('/console/dashboard')}
        >
          <div className="h-8 w-8 shrink-0 rounded-lg bg-primary flex items-center justify-center font-display font-extrabold text-[13px] text-primary-content tracking-tight">
            GW
          </div>
          <span className={`font-display font-bold text-[15px] transition-opacity duration-200 ${collapsed ? 'opacity-0' : 'opacity-100'}`}>
            LLM Gateway
          </span>
        </div>

        {/* Navigation */}
        <nav className="flex-1 overflow-y-auto overflow-x-hidden px-2 py-3 flex flex-col gap-0.5">
          {!collapsed && (
            <div className="text-[10px] font-semibold uppercase tracking-[0.12em] text-base-content/40 px-3 pt-4 pb-1.5">
              Console
            </div>
          )}
          {consoleItems.map((item) => {
            const Icon = item.icon;
            const active = location.pathname === item.key;
            return (
              <div
                key={item.key}
                className={`flex items-center gap-3 rounded-lg px-3 py-2 cursor-pointer text-sm font-medium transition-all duration-200 whitespace-nowrap overflow-hidden select-none ${
                  active ? 'bg-primary/10 text-primary' : 'text-base-content/60 hover:bg-base-300 hover:text-base-content'
                } ${collapsed ? 'justify-center px-2' : ''}`}
                onClick={() => navigate(item.key)}
              >
                <Icon className="h-[17px] w-[17px] shrink-0" />
                <span className={collapsed ? 'opacity-0 pointer-events-none' : ''}>{item.label}</span>
              </div>
            );
          })}

          {isAdmin && (
            <>
              {!collapsed && (
                <div className="text-[10px] font-semibold uppercase tracking-[0.12em] text-base-content/40 px-3 pt-5 pb-1.5">
                  Admin
                </div>
              )}
              {adminItems.map((item) => {
                const Icon = item.icon;
                const active = location.pathname === item.key;
                return (
                  <div
                    key={item.key}
                    className={`flex items-center gap-3 rounded-lg px-3 py-2 cursor-pointer text-sm font-medium transition-all duration-200 whitespace-nowrap overflow-hidden select-none ${
                      active ? 'bg-primary/10 text-primary' : 'text-base-content/60 hover:bg-base-300 hover:text-base-content'
                    } ${collapsed ? 'justify-center px-2' : ''}`}
                    onClick={() => navigate(item.key)}
                  >
                    <Icon className="h-[17px] w-[17px] shrink-0" />
                    <span className={collapsed ? 'opacity-0 pointer-events-none' : ''}>{item.label}</span>
                  </div>
                );
              })}
            </>
          )}
        </nav>

        {/* Version + Collapse toggle */}
        <div className="border-t border-base-300 p-2">
          {!collapsed && version && (
            <div className="text-[10px] text-base-content/30 font-mono px-2 pb-2 truncate">
              v{version}
            </div>
          )}
          <button
            className="btn btn-ghost btn-sm btn-square w-full"
            onClick={() => setCollapsed(!collapsed)}
          >
            {collapsed ? <PanelLeftOpen className="h-4 w-4" /> : <PanelLeftClose className="h-4 w-4" />}
          </button>
        </div>
      </aside>

      {/* Main content */}
      <div className={`flex min-h-screen flex-col transition-all duration-300 ${collapsed ? 'ml-[72px]' : 'ml-[240px]'}`}>
        {/* Header */}
        <header className="sticky top-0 z-50 flex h-14 items-center justify-between border-b border-base-300 bg-base-100/95 backdrop-blur px-6 gap-3 shrink-0">
          {/* Left: Breadcrumb */}
          <div className="text-xs font-mono text-base-content/50 uppercase tracking-wider">
            {location.pathname.replace('/console/', '').replace(/\//g, ' / ')}
          </div>

          {/* Right: Theme toggle + User */}
          <div className="flex items-center gap-2">
            <button
              className="btn btn-ghost btn-sm btn-circle"
              onClick={toggleTheme}
              aria-label="Toggle theme"
            >
              {theme === 'dark' ? <Sun className="h-4 w-4" /> : <Moon className="h-4 w-4" />}
            </button>

            <div className="flex items-center gap-2 text-sm font-medium text-base-content/70">
              <div className="avatar placeholder">
                <div className="bg-primary text-primary-content w-7 rounded-md">
                  <span className="text-xs font-semibold">{user?.username?.charAt(0).toUpperCase()}</span>
                </div>
              </div>
              <span className="hidden sm:inline">{user?.username}</span>
            </div>
            <button
              className="btn btn-ghost btn-sm gap-1.5 text-base-content/50"
              onClick={logout}
            >
              <LogOut className="h-3.5 w-3.5" />
              <span className="hidden sm:inline">Logout</span>
            </button>
          </div>
        </header>

        {/* Content */}
        <main className="flex-1 bg-base-200/50 p-6">
          <div className="animate-fade-in-up">
            <Outlet />
          </div>
        </main>

        {/* Footer */}
        <footer className="border-t border-base-300 px-6 py-3 text-center text-[11px] text-base-content/30 font-mono shrink-0">
          LLM Gateway{version ? ` v${version}` : ''}
        </footer>
      </div>
    </div>
  );
}
