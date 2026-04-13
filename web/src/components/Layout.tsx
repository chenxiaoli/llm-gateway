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
        'fixed left-0 top-0 bottom-0 z-[100] flex flex-col transition-all duration-300 overflow-hidden sidebar-glow',
        collapsed ? 'w-[72px]' : 'w-[240px]',
      )}
      style={{ background: 'linear-gradient(180deg, #0c0c0c 0%, #080808 100%)', borderRight: '1px solid rgba(30, 30, 30, 0.8)' }}
      >
        {/* Logo */}
        <div
          className="flex h-14 items-center gap-3 px-4 cursor-pointer overflow-hidden whitespace-nowrap"
          style={{ borderBottom: '1px solid rgba(30, 30, 30, 0.6)' }}
          onClick={() => navigate('/console/dashboard')}
        >
          <div
            className="h-8 w-8 shrink-0 rounded-lg flex items-center justify-center font-display font-extrabold text-[13px] text-black tracking-tight"
            style={{
              background: 'linear-gradient(135deg, #06d6a0 0%, #059669 100%)',
              boxShadow: '0 0 16px rgba(6, 214, 160, 0.25)',
            }}
          >
            GW
          </div>
          <span className={cn(
            'font-display font-bold text-[15px] text-[#ededed] transition-opacity duration-200',
            collapsed ? 'opacity-0 pointer-events-none' : 'opacity-100',
          )}>
            LLM Gateway
          </span>
        </div>

        {/* Navigation */}
        <nav className="flex-1 overflow-y-auto overflow-x-hidden px-2 py-3 flex flex-col gap-0.5">
          <div className={cn(
            'text-[10px] font-semibold uppercase tracking-[0.12em] text-[#666666] px-3 pt-4 pb-1.5 whitespace-nowrap transition-all duration-200',
            collapsed ? 'opacity-0 h-0 p-0 overflow-hidden' : 'opacity-100',
          )}>
            Console
          </div>
          {consoleItems.map((item, i) => {
            const Icon = item.icon;
            const active = location.pathname === item.key;
            return (
              <div
                key={item.key}
                className={cn(
                  'flex items-center gap-3 rounded-lg px-3 py-2 cursor-pointer text-sm font-medium transition-all duration-200 whitespace-nowrap overflow-hidden select-none',
                  active
                    ? 'bg-accent/[0.08] text-accent'
                    : 'text-[#888888] hover:bg-white/[0.04] hover:text-[#cccccc]',
                  collapsed && 'justify-center px-2',
                  active && !collapsed && 'nav-active',
                )}
                style={active ? { animationDelay: `${i * 40}ms` } : undefined}
                onClick={() => navigate(item.key)}
              >
                <Icon className={cn(
                  'h-[17px] w-[17px] shrink-0 transition-colors duration-200',
                  active && 'drop-shadow-[0_0_6px_rgba(6,214,160,0.4)]',
                )} />
                <span className={cn(collapsed && 'opacity-0 pointer-events-none')}>{item.label}</span>
              </div>
            );
          })}

          {isAdmin && (
            <>
              <div className={cn(
                'text-[10px] font-semibold uppercase tracking-[0.12em] text-[#666666] px-3 pt-5 pb-1.5 whitespace-nowrap transition-all duration-200',
                collapsed ? 'opacity-0 h-0 p-0 overflow-hidden' : 'opacity-100',
              )}>
                Admin
              </div>
              {adminItems.map((item, i) => {
                const Icon = item.icon;
                const active = location.pathname === item.key;
                return (
                  <div
                    key={item.key}
                    className={cn(
                      'flex items-center gap-3 rounded-lg px-3 py-2 cursor-pointer text-sm font-medium transition-all duration-200 whitespace-nowrap overflow-hidden select-none',
                      active
                        ? 'bg-accent/[0.08] text-accent'
                        : 'text-[#888888] hover:bg-white/[0.04] hover:text-[#cccccc]',
                      collapsed && 'justify-center px-2',
                      active && !collapsed && 'nav-active',
                    )}
                    style={active ? { animationDelay: `${i * 40}ms` } : undefined}
                    onClick={() => navigate(item.key)}
                  >
                    <Icon className={cn(
                      'h-[17px] w-[17px] shrink-0 transition-colors duration-200',
                      active && 'drop-shadow-[0_0_6px_rgba(6,214,160,0.4)]',
                    )} />
                    <span className={cn(collapsed && 'opacity-0 pointer-events-none')}>{item.label}</span>
                  </div>
                );
              })}
            </>
          )}
        </nav>

        {/* Version + Collapse toggle */}
        <div style={{ borderTop: '1px solid rgba(30, 30, 30, 0.6)' }} className="p-2">
          {!collapsed && version && (
            <div className="text-[10px] text-[#555555] font-mono px-2 pb-2 truncate">
              v{version}
            </div>
          )}
          <button
            className="flex w-full items-center justify-center rounded-lg p-2 text-[#666666] hover:bg-white/[0.04] hover:text-[#999999] transition-colors duration-200"
            onClick={() => setCollapsed(!collapsed)}
          >
            {collapsed ? <PanelLeftOpen className="h-4 w-4" /> : <PanelLeftClose className="h-4 w-4" />}
          </button>
        </div>
      </aside>

      {/* Main content */}
      <div className={cn(
        'flex min-h-screen flex-col transition-all duration-300',
        collapsed ? 'ml-[72px]' : 'ml-[240px]',
      )}>
        {/* Header */}
        <header
          className="sticky top-0 z-50 flex h-14 items-center justify-between border-b px-6 gap-3 shrink-0 backdrop-blur-md"
          style={{
            borderColor: 'rgba(30, 30, 30, 0.8)',
            background: 'rgba(0, 0, 0, 0.8)',
          }}
        >
          {/* Left: Current page breadcrumb */}
          <div className="flex items-center gap-2">
            <div className="pulse-dot h-1.5 w-1.5 rounded-full bg-accent" />
            <span className="text-[11px] font-mono text-[#777777] uppercase tracking-wider">
              {location.pathname.replace('/console/', '').replace(/\//g, ' / ')}
            </span>
          </div>

          {/* Right: User */}
          <div className="flex items-center gap-3">
            <div className="flex items-center gap-2 text-sm font-medium text-[#888888]">
              <div
                className="flex h-7 w-7 items-center justify-center rounded-md font-display font-semibold text-xs"
                style={{
                  background: 'linear-gradient(135deg, rgba(6, 214, 160, 0.15) 0%, rgba(6, 214, 160, 0.05) 100%)',
                  color: '#06d6a0',
                  border: '1px solid rgba(6, 214, 160, 0.2)',
                }}
              >
                {user?.username?.charAt(0).toUpperCase()}
              </div>
              <span className="hidden sm:inline">{user?.username}</span>
            </div>
            <div className="h-5 w-px bg-[#1e1e1e]" />
            <button
              className="flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-[13px] text-[#777777] hover:bg-white/[0.04] hover:text-[#aaaaaa] transition-colors duration-200"
              onClick={logout}
            >
              <LogOut className="h-3.5 w-3.5" />
              <span className="hidden sm:inline">Logout</span>
            </button>
          </div>
        </header>

        {/* Content */}
        <main
          className="flex-1 relative p-6 overflow-auto"
          style={{
            background: 'radial-gradient(ellipse at 50% 0%, rgba(6, 214, 160, 0.02) 0%, transparent 50%)',
          }}
        >
          <div className="animate-fade-in-up relative z-10">
            <Outlet />
          </div>
        </main>

        {/* Footer */}
        <footer
          className="px-6 py-3 text-center text-[11px] text-[#555555] font-mono shrink-0"
          style={{ borderTop: '1px solid rgba(30, 30, 30, 0.6)' }}
        >
          LLM Gateway{version ? ` v${version}` : ''}
        </footer>
      </div>
    </div>
  );
}
