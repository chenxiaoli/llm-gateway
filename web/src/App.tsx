import { BrowserRouter, Routes, Route, Navigate, Outlet } from 'react-router-dom';
import { useAuthStore, useAuthBootstrap } from './stores/authStore';
import { getToken } from './api/client';
import { isAdminOrAbove } from './lib/auth';
import { LoadingSpinner } from './components/ui/LoadingSpinner';
import Layout from './components/Layout';
import { OrgRouteGuard } from './components/OrgRouteGuard';
import { LegacyRedirect } from './components/LegacyRedirect';
import Home from './pages/Home';
import Login from './pages/Login';
import Register from './pages/Register';
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
import Groups from './pages/Groups';
import AccountBalance from './pages/AccountBalance';
import Settings from './pages/Settings';
import Usage from './pages/Usage';
import Logs from './pages/Logs';
import AdminDashboard from './pages/AdminDashboard';
import DocsLayout from './pages/DocsLayout';
import DocsPage from './pages/DocsPage';

function RequireAuth() {
  const user = useAuthStore((s) => s.user);
  const { isLoading } = useAuthBootstrap();
  if (isLoading) return <div className="flex h-screen items-center justify-center"><LoadingSpinner size="lg" /></div>;
  if (!user) {
    if (getToken()) return <div className="flex h-screen items-center justify-center"><LoadingSpinner size="lg" /></div>;
    return <Navigate to="/login" replace />;
  }
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

function App() {
  return (
    <BrowserRouter>
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
