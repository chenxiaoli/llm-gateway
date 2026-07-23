import { Outlet, useNavigate, useLocation } from 'react-router-dom';
import { useAuthStore } from '../stores/authStore';
import { isPlatformAdmin } from '../lib/auth';
import { displayName } from '../lib/displayName';
import { useTranslation } from 'react-i18next';
import { Settings, Users, PanelLeftClose } from 'lucide-react';

export default function PlatformLayout() {
  const navigate = useNavigate();
  const location = useLocation();
  const user = useAuthStore((s) => s.user);
  const currentOrg = useAuthStore((s) => s.currentOrg);
  const { t } = useTranslation();

  // The route guard at App.tsx already enforces this; render-time
  // double-check prevents flicker if a stale render slips through.
  if (!isPlatformAdmin(user)) return null;

  const navItem = (path: string, label: string, Icon: typeof Settings, active: boolean) => (
    <button
      key={path}
      type="button"
      className={`group/nav flex items-center gap-3 rounded-lg px-3 py-2 cursor-pointer text-base font-medium transition-all duration-150 whitespace-nowrap overflow-hidden select-none relative ${
        active
          ? 'bg-primary/10 text-primary'
          : 'text-base-content/50 hover:bg-base-200 hover:text-base-content/80'
      }`}
      onClick={() => navigate(path)}
    >
      {active && (
        <div className="absolute left-0 top-1/2 -translate-y-1/2 w-[3px] h-4 rounded-r-full bg-primary" />
      )}
      <Icon className="h-[17px] w-[17px] shrink-0" strokeWidth={active ? 2 : 1.5} />
      <span>{label}</span>
    </button>
  );

  const items = [
    { path: '/admin/settings', label: t('sidebar.settings'), Icon: Settings },
    { path: '/admin/platform-users', label: t('sidebar.platformUsers'), Icon: Users },
  ];

  return (
    <div className="flex min-h-screen bg-base-200">
      <aside className="w-[232px] fixed left-0 top-0 bottom-0 z-[100] flex flex-col bg-base-100 border-r border-base-300/60">
        <div className="flex h-14 items-center gap-3 border-b border-base-300/60 px-4">
          <div className="h-8 w-8 shrink-0 rounded-lg bg-primary flex items-center justify-center font-semibold text-md text-primary-content tracking-tight">
            TV
          </div>
          <span className="font-semibold text-lg">TokenVis</span>
        </div>
        <nav className="flex-1 overflow-y-auto px-3 py-4 flex flex-col gap-0.5">
          <div className="text-xs font-semibold uppercase tracking-[0.12em] text-base-content/30 px-3 pt-1 pb-2">
            {t('sidebar.platform')}
          </div>
          {items.map(({ path, label, Icon }) =>
            navItem(path, label, Icon, location.pathname === path || location.pathname.startsWith(path + '/')),
          )}
        </nav>
      </aside>

      <div className="flex min-h-screen flex-col md:ml-[232px]">
        <header className="fixed top-0 z-40 shrink-0 bg-base-100/80 backdrop-blur-md border-b border-base-300/60 h-12 left-0 md:left-[232px] right-0">
          <div className="flex h-12 items-center px-4 md:px-6 gap-3 w-full">
            {currentOrg && (
              <button
                type="button"
                onClick={() => navigate(`/${currentOrg.slug}/dashboard`)}
                className="text-sm text-base-content/60 hover:text-base-content transition-colors flex items-center gap-1.5"
              >
                <PanelLeftClose className="h-3.5 w-3.5" />
                {t('platformLayout.backTo')} {currentOrg.name}
              </button>
            )}
            <div className="ml-auto text-xs text-base-content/40">{user ? displayName(user) : ''}</div>
          </div>
        </header>
        <main className="flex-1 p-4 md:p-6 overflow-y-auto pt-16 pb-8">
          <div className="animate-fade-in-up" key={location.pathname}>
            <Outlet />
          </div>
        </main>
      </div>
    </div>
  );
}
