import { BrowserRouter, Routes, Route, Navigate, Outlet, useLocation, useNavigate } from 'react-router-dom';
import { useEffect } from 'react';
import { useAuthStore, useAuthBootstrap, useAuthGate } from './stores/authStore';
import { getToken } from './api/client';
import { isAdminOrAbove } from './lib/auth';
import { LoadingSpinner } from './components/ui/LoadingSpinner';
import Layout from './components/Layout';
import { OrgRouteGuard } from './components/OrgRouteGuard';
import { LegacyRedirect } from './components/LegacyRedirect';
import Home from './pages/Home';
import Login from './pages/Login';
import Register from './pages/Register';
import Onboarding from './pages/Onboarding';
import AcceptInvite from './pages/AcceptInvite';
import Dashboard from './pages/Dashboard';
import Account from './pages/Account';
import ChangePassword from './pages/ChangePassword';
import Keys from './pages/Keys';
import KeyDetail from './pages/KeyDetail';
import ModelFallbacks from './pages/ModelFallbacks';
import ConsoleModels from './pages/ConsoleModels';
import Channels from './pages/Channels';
import ChannelDetail from './pages/ChannelDetail';
import Models from './pages/Models';
import PricingPolicies from './pages/PricingPolicies';
import Providers from './pages/Providers';
import ProviderDetail from './pages/ProviderDetail';
import Users from './pages/Users';
import Members from './pages/Members';
import Invitations from './pages/Invitations';
import OrgSettings from './pages/OrgSettings';
import Groups from './pages/Groups';
import AccountBalance from './pages/AccountBalance';
import Settings from './pages/Settings';
import Usage from './pages/Usage';
import Logs from './pages/Logs';
import AdminDashboard from './pages/AdminDashboard';
import DocsLayout from './pages/DocsLayout';
import DocsPage from './pages/DocsPage';

function RequireAuth() {
  const status = useAuthGate();
  if (status === 'loading') return <div className="flex h-screen items-center justify-center"><LoadingSpinner size="lg" /></div>;
  if (status === 'login') return <Navigate to="/login" replace />;
  return <Outlet />;
}

function RequireAdmin() {
  const user = useAuthStore((s) => s.user);
  const currentOrg = useAuthStore((s) => s.currentOrg);
  const { isLoading } = useAuthBootstrap();
  if (isLoading) return <div className="flex h-screen items-center justify-center"><LoadingSpinner size="lg" /></div>;
  if (!user) {
    if (getToken()) return <div className="flex h-screen items-center justify-center"><LoadingSpinner size="lg" /></div>;
    return <Navigate to="/login" replace />;
  }
  if (!isAdminOrAbove(user, currentOrg)) {
    const slug = currentOrg?.slug;
    return <Navigate to={slug ? `/${slug}/dashboard` : '/login'} replace />;
  }
  return <Outlet />;
}

/**
 * Limbo-user redirect: any authenticated user with zero org memberships who
 * is NOT already on /onboarding or /accept-invite gets bounced to /onboarding.
 * Mounted at the router root (outside <Routes>) so it can observe every
 * location change. This is the global safety net — the per-card flows in
 * Onboarding.tsx are responsible for navigating away after a success.
 */
function OnboardingRedirect() {
  const user = useAuthStore((s) => s.user);
  const orgs = useAuthStore((s) => s.orgs);
  const { isLoading } = useAuthBootstrap();
  const location = useLocation();
  const navigate = useNavigate();
  useEffect(() => {
    // During cold-load bootstrap, user may be populated from a prior session
    // while orgs is still the stale []. Bail until bootstrap settles so we
    // don't bounce a user who actually has orgs.
    if (isLoading) return;
    if (!user || orgs.length > 0) return;
    const path = location.pathname;
    const onAllowedPath = path === '/onboarding' || path.startsWith('/accept-invite');
    if (!onAllowedPath) {
      navigate('/onboarding', { replace: true });
    }
  }, [isLoading, user, orgs, location.pathname, navigate]);
  return null;
}

function App() {
  return (
    <BrowserRouter>
      <OnboardingRedirect />
      <Routes>
        <Route path="/" element={<Home />} />
        <Route path="/docs" element={<DocsLayout />}>
          <Route
            index
            element={<Navigate to={`${localStorage.getItem('i18n-language') === 'en' ? 'en' : 'zh'}/user/getting-started`} replace />}
          />
          <Route path=":lang/:section/:slug" element={<DocsPage />} />
        </Route>

        {/* User-scoped — no org prefix, no Layout */}
        <Route path="/login" element={<Login />} />
        <Route path="/register" element={<Register />} />
        <Route path="/accept-invite" element={<AcceptInvite />} />
        <Route path="/onboarding" element={<Onboarding />} />

        {/* Org-scoped — wraps everything else */}
        <Route path="/:orgSlug" element={<Layout />}>
          <Route element={<RequireAuth />}>
            <Route element={<OrgRouteGuard />}>
              <Route index element={<Navigate to="dashboard" replace />} />
              <Route path="dashboard" element={<Dashboard />} />
              <Route path="keys" element={<Keys />} />
              <Route path="keys/:id" element={<KeyDetail />} />
              <Route path="model-fallbacks" element={<ModelFallbacks />} />
              <Route path="models" element={<ConsoleModels />} />
              <Route path="usage" element={<Usage />} />
              <Route path="members" element={<Members />} />
              <Route path="settings" element={<OrgSettings />} />
              <Route path="account" element={<Account />} />
              <Route path="change-password" element={<ChangePassword />} />
            </Route>
          </Route>

          <Route element={<RequireAdmin />}>
            <Route element={<OrgRouteGuard />}>
              <Route path="admin/dashboard" element={<AdminDashboard />} />
              <Route path="admin/channels" element={<Channels />} />
              <Route path="admin/channels/:id" element={<ChannelDetail />} />
              <Route path="admin/providers" element={<Providers />} />
              <Route path="admin/providers/:id" element={<ProviderDetail />} />
              <Route path="admin/models" element={<Models />} />
              <Route path="admin/pricing-policies" element={<PricingPolicies />} />
              <Route path="admin/users" element={<Users />} />
              <Route path="admin/invitations" element={<Invitations />} />
              <Route path="admin/groups" element={<Groups />} />
              <Route path="admin/users/:userId/balance" element={<AccountBalance />} />
              <Route path="admin/settings" element={<Settings />} />
              <Route path="admin/logs" element={<Logs />} />
            </Route>
          </Route>
        </Route>

        {/* Legacy paths — redirect to current org */}
        <Route path="/console/*" element={<LegacyRedirect />} />
        <Route path="/admin/*" element={<LegacyRedirect />} />

        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </BrowserRouter>
  );
}

export default App;
