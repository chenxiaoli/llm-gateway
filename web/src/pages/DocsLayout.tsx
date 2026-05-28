import { useState } from 'react';
import { Outlet, useNavigate, useLocation } from 'react-router-dom';
import { Sun, Moon } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useTheme } from '../hooks/useTheme';
import { Button } from '../components/ui/Button';
import { getToken } from '../api/client';
import { ChevronRight, Menu, X } from 'lucide-react';

const docsNav = {
  user: [
    { title: '快速开始', slug: 'getting-started' },
    { title: 'API 密钥管理', slug: 'api-keys' },
    { title: '余额充值', slug: 'balance' },
    { title: '用量统计', slug: 'usage' },
  ],
  admin: [
    { title: '渠道配置', slug: 'channels' },
    { title: '供应商管理', slug: 'providers' },
    { title: '模型管理', slug: 'models' },
    { title: '定价策略', slug: 'pricing-policies' },
    { title: '费率限制', slug: 'rate-limits' },
  ],
};

export default function DocsLayout() {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const location = useLocation();
  const { theme, toggleTheme } = useTheme();
  const [sidebarOpen, setSidebarOpen] = useState(false);

  const toggleLanguage = () => {
    const next = i18n.language === 'zh' ? 'en' : 'zh';
    i18n.changeLanguage(next);
    localStorage.setItem('i18n-language', next);
  };

  const isActive = (section: string, slug: string) => location.pathname === `/docs/${section}/${slug}`;

  return (
    <div className="min-h-screen bg-base-200">
      {/* Header */}
      <header className="fixed top-0 inset-x-0 z-50 border-b border-base-300/40 bg-base-200/80 backdrop-blur-md">
        <div className="flex h-14 items-center justify-between px-6 max-w-7xl mx-auto">
          <button onClick={() => navigate('/')} className="flex items-center gap-2.5">
            <div className="h-8 w-8 rounded-lg bg-primary flex items-center justify-center text-primary-content font-bold text-sm">GW</div>
            <span className="font-semibold text-lg">{t('home.brand')}</span>
          </button>
          <div className="flex items-center gap-2">
            <button className="btn btn-ghost btn-sm btn-circle" onClick={toggleLanguage} title={i18n.language === 'zh' ? 'Switch to English' : '切换到中文'}>
              {i18n.language === 'zh' ? 'EN' : '中'}
            </button>
            <button className="btn btn-ghost btn-sm btn-circle" onClick={toggleTheme} aria-label="Toggle theme">
              {theme === 'dark' ? <Sun className="h-4 w-4" /> : <Moon className="h-4 w-4" />}
            </button>
            <Button variant="primary" size="sm" onClick={() => navigate(getToken() ? '/console/dashboard' : '/console/login')}>
              {t('home.dashboard')}
            </Button>
          </div>
        </div>
      </header>

      <div className="flex pt-14">
        {/* Mobile overlay */}
        {sidebarOpen && (
          <div className="fixed inset-0 z-40 bg-black/50 md:hidden" onClick={() => setSidebarOpen(false)} />
        )}

        {/* Sidebar */}
        <aside className={`
          fixed md:static inset-y-14 left-0 z-50 w-72 bg-base-200 border-r border-base-300/40
          transform transition-transform duration-200 ease-in-out
          ${sidebarOpen ? 'translate-x-0' : '-translate-x-full md:translate-x-0'}
          overflow-y-auto
        `}>
          <div className="p-6">
            {/* User Guide */}
            <div className="mb-6">
              <h3 className="text-xs font-semibold uppercase tracking-widest text-base-content/40 mb-3">
                {t('docs.userGuide')}
              </h3>
              <ul className="space-y-1">
                {docsNav.user.map((item) => (
                  <li key={item.slug}>
                    <button
                      onClick={() => { navigate(`/docs/user/${item.slug}`); setSidebarOpen(false); }}
                      className={`
                        w-full text-left px-3 py-2 rounded-lg text-sm transition-all
                        ${isActive('user', item.slug)
                          ? 'bg-primary/10 text-primary font-medium'
                          : 'text-base-content/60 hover:text-base-content hover:bg-base-300/40'}
                      `}
                    >
                      {item.title}
                    </button>
                  </li>
                ))}
              </ul>
            </div>

            {/* Admin Guide */}
            <div>
              <h3 className="text-xs font-semibold uppercase tracking-widest text-base-content/40 mb-3">
                {t('docs.adminGuide')}
              </h3>
              <ul className="space-y-1">
                {docsNav.admin.map((item) => (
                  <li key={item.slug}>
                    <button
                      onClick={() => { navigate(`/docs/admin/${item.slug}`); setSidebarOpen(false); }}
                      className={`
                        w-full text-left px-3 py-2 rounded-lg text-sm transition-all flex items-center gap-1
                        ${isActive('admin', item.slug)
                          ? 'bg-primary/10 text-primary font-medium'
                          : 'text-base-content/60 hover:text-base-content hover:bg-base-300/40'}
                      `}
                    >
                      <ChevronRight className="h-3 w-3 shrink-0" />
                      {item.title}
                    </button>
                  </li>
                ))}
              </ul>
            </div>
          </div>
        </aside>

        {/* Mobile toggle */}
        <button
          className="fixed bottom-4 left-4 z-50 btn btn-circle btn-primary md:hidden shadow-lg"
          onClick={() => setSidebarOpen(!sidebarOpen)}
        >
          {sidebarOpen ? <X className="h-4 w-4" /> : <Menu className="h-4 w-4" />}
        </button>

        {/* Content */}
        <main className="flex-1 min-w-0">
          <div className="max-w-3xl mx-auto p-6 md:p-10">
            <Outlet />
          </div>
        </main>
      </div>
    </div>
  );
}