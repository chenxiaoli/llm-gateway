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
import Channels from './pages/Channels';
import ChannelDetail from './pages/ChannelDetail';
import Models from './pages/Models';
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
            <Route path="channels" element={<Channels />} />
            <Route path="channels/:id" element={<ChannelDetail />} />
            <Route path="models" element={<Models />} />
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
