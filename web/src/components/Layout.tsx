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
