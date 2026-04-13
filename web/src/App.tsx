import { BrowserRouter, Routes, Route, Navigate, Outlet } from 'react-router-dom';
import { Spin } from 'antd';
import { AuthProvider, useAuth } from './contexts/AuthContext';
import { getToken } from './api/client';
import Layout from './components/Layout';
import Home from './pages/Home';
import Login from './pages/Login';
import Register from './pages/Register';
import Dashboard from './pages/Dashboard';
import Keys from './pages/Keys';
import KeyDetail from './pages/KeyDetail';
import Providers from './pages/Providers';
import ProviderDetail from './pages/ProviderDetail';
import Users from './pages/Users';
import Settings from './pages/Settings';
import Usage from './pages/Usage';
import Logs from './pages/Logs';

function RequireAuth() {
  const { user, isLoading } = useAuth();
  if (isLoading) return <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '100vh' }}><Spin size="large" /></div>;
  if (!user) {
    if (getToken()) return <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '100vh' }}><Spin size="large" /></div>;
    return <Navigate to="/console/login" replace />;
  }
  return <Outlet />;
}

function RequireAdmin() {
  const { user, isLoading } = useAuth();
  if (isLoading) return <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '100vh' }}><Spin size="large" /></div>;
  if (!user || user.role !== 'admin') return <Navigate to="/console/dashboard" replace />;
  return <Outlet />;
}

function App() {
  return (
    <BrowserRouter>
      <AuthProvider>
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
              <Route path="users" element={<Users />} />
              <Route path="settings" element={<Settings />} />
              <Route path="logs" element={<Logs />} />
            </Route>
            <Route index element={<Navigate to="/console/dashboard" replace />} />
          </Route>
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </AuthProvider>
    </BrowserRouter>
  );
}

export default App;
