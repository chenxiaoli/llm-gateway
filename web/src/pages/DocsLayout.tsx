import { useState, useEffect } from 'react';
import { Outlet, useNavigate, useLocation, Link } from 'react-router-dom';
import { Sun, Moon } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useTheme } from '../hooks/useTheme';
import { Button } from '../components/ui/Button';
import { getToken } from '../api/client';
import { Menu, X } from 'lucide-react';

const docsNav = {
  user: [
    { titleKey: 'docs.nav.gettingStarted', slug: 'getting-started' },
    { titleKey: 'docs.nav.apiKeys', slug: 'api-keys' },
    { titleKey: 'docs.nav.balance', slug: 'balance' },
    { titleKey: 'docs.nav.usage', slug: 'usage' },
  ],
  admin: [
    { titleKey: 'docs.nav.channels', slug: 'channels' },
    { titleKey: 'docs.nav.providers', slug: 'providers' },
    { titleKey: 'docs.nav.models', slug: 'models' },
    { titleKey: 'docs.nav.pricingPolicies', slug: 'pricing-policies' },
    { titleKey: 'docs.nav.rateLimits', slug: 'rate-limits' },
  ],
};

export default function DocsLayout() {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const location = useLocation();
  const { theme, toggleTheme } = useTheme();
  const [sidebarOpen, setSidebarOpen] = useState(false);

  const langMatch = location.pathname.match(/\/docs\/(zh|en)\//);
  const currentLang = langMatch ? langMatch[1] : (i18n.language === 'en' ? 'en' : 'zh');

  // Sync i18n language to URL lang so sidebar text matches
  useEffect(() => {
    if (langMatch && langMatch[1] !== i18n.language) {
      i18n.changeLanguage(langMatch[1]);
      localStorage.setItem('i18n-language', langMatch[1]);
    }
  }, [langMatch, i18n]);

  const toggleLanguage = () => {
    const next = currentLang === 'zh' ? 'en' : 'zh';
    i18n.changeLanguage(next);
    localStorage.setItem('i18n-language', next);
    const currentPath = location.pathname;
    const newPath = currentPath.replace(/\/docs\/(zh|en)\//, `/docs/${next}/`);
    if (newPath !== currentPath) {
      navigate(newPath, { replace: true });
    }
  };

  const isActive = (section: string, slug: string) =>
    location.pathname === `/docs/${currentLang}/${section}/${slug}`;

  return (
    <div className="min-h-screen bg-base-200">
      <header className="fixed top-0 inset-x-0 z-50 border-b border-base-300/40 bg-base-200/80 backdrop-blur-md">
        <div className="max-w-6xl mx-auto h-14 flex items-center justify-between px-6">
          <div className="flex items-center gap-2.5">
            <button onClick={() => navigate('/')} className="flex items-center gap-2.5">
              <div className="h-8 w-8 rounded-lg bg-primary flex items-center justify-center text-primary-content font-bold text-sm">TV</div>
              <span className="font-semibold text-lg">{t('home.brand')}</span>
            </button>
          </div>
          <div className="flex items-center gap-2">
            <button
              className="btn btn-ghost btn-sm btn-circle"
              onClick={toggleLanguage}
              aria-label={currentLang === 'zh' ? 'Switch to English' : '切换到中文'}
            >
              {currentLang === 'zh' ? 'EN' : '中'}
            </button>
            <button
              className="btn btn-ghost btn-sm btn-circle"
              onClick={toggleTheme}
              aria-label="Toggle theme"
            >
              {theme === 'dark' ? <Sun className="h-4 w-4" /> : <Moon className="h-4 w-4" />}
            </button>
            <Button variant="primary" size="sm" onClick={() => navigate(getToken() ? '/console/dashboard' : '/console/login')}>
              {t('home.dashboard')}
            </Button>
          </div>
        </div>
      </header>

      <div className="max-w-6xl mx-auto flex pt-14">
        {sidebarOpen && (
          <div className="fixed inset-0 z-40 bg-black/50 md:hidden" onClick={() => setSidebarOpen(false)} />
        )}

        <aside className={`
          fixed md:sticky top-14 left-0 z-50 w-60 shrink-0
          border-r border-base-300/40 bg-base-200
          h-[calc(100vh-3.5rem)] overflow-y-auto
          transition-transform duration-200 ease-in-out
          ${sidebarOpen ? 'translate-x-0' : '-translate-x-full md:translate-x-0'}
        `}>
          <nav className="p-4" aria-label="Documentation navigation">
            <div className="mb-6">
              <h3 className="text-[11px] font-semibold uppercase tracking-wider text-base-content/40 mb-2 px-3">
                {t('home.docs.userGuide')}
              </h3>
              <ul className="space-y-0.5">
                {docsNav.user.map((item) => (
                  <li key={item.slug}>
                    <Link
                      to={`/docs/${currentLang}/user/${item.slug}`}
                      onClick={() => setSidebarOpen(false)}
                      aria-current={isActive('user', item.slug) ? 'page' : undefined}
                      className={`
                        block px-3 py-1.5 rounded-md text-sm transition-colors duration-150
                        ${isActive('user', item.slug)
                          ? 'bg-primary/10 text-primary font-medium'
                          : 'text-base-content/60 hover:text-base-content hover:bg-base-300/40'}
                      `}
                    >
                      {t(item.titleKey)}
                    </Link>
                  </li>
                ))}
              </ul>
            </div>

            <div>
              <h3 className="text-[11px] font-semibold uppercase tracking-wider text-base-content/40 mb-2 px-3">
                {t('home.docs.adminGuide')}
              </h3>
              <ul className="space-y-0.5">
                {docsNav.admin.map((item) => (
                  <li key={item.slug}>
                    <Link
                      to={`/docs/${currentLang}/admin/${item.slug}`}
                      onClick={() => setSidebarOpen(false)}
                      aria-current={isActive('admin', item.slug) ? 'page' : undefined}
                      className={`
                        block px-3 py-1.5 rounded-md text-sm transition-colors duration-150
                        ${isActive('admin', item.slug)
                          ? 'bg-primary/10 text-primary font-medium'
                          : 'text-base-content/60 hover:text-base-content hover:bg-base-300/40'}
                      `}
                    >
                      {t(item.titleKey)}
                    </Link>
                  </li>
                ))}
              </ul>
            </div>
          </nav>
        </aside>

        <button
          className="fixed bottom-4 left-4 z-50 btn btn-circle btn-primary md:hidden shadow-lg"
          onClick={() => setSidebarOpen(!sidebarOpen)}
          aria-label={sidebarOpen ? 'Close navigation' : 'Open navigation'}
        >
          {sidebarOpen ? <X className="h-4 w-4" /> : <Menu className="h-4 w-4" />}
        </button>

        <main className="flex-1 min-w-0 min-h-[calc(100vh-3.5rem)]">
          <div className="p-6 md:p-10">
            <Outlet />
          </div>
        </main>
      </div>
    </div>
  );
}
