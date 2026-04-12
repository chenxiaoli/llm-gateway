import { BrowserRouter, Routes, Route, Navigate, Outlet } from 'react-router-dom';
import { getToken } from './api/client';
import Layout from './components/Layout';
import Home from './pages/Home';
import Login from './pages/Login';
import Dashboard from './pages/Dashboard';
import Keys from './pages/Keys';
import KeyDetail from './pages/KeyDetail';
import Providers from './pages/Providers';
import ProviderDetail from './pages/ProviderDetail';
import Usage from './pages/Usage';
import Logs from './pages/Logs';

function RequireAuth() {
  const token = getToken();
  if (!token) return <Navigate to="/admin/login" replace />;
  return <Outlet />;
}

function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<Home />} />
        <Route path="/admin/login" element={<Login />} />
        <Route path="/admin" element={<Layout />}>
          <Route element={<RequireAuth />}>
            <Route path="dashboard" element={<Dashboard />} />
            <Route path="keys" element={<Keys />} />
            <Route path="keys/:id" element={<KeyDetail />} />
            <Route path="providers" element={<Providers />} />
            <Route path="providers/:id" element={<ProviderDetail />} />
            <Route path="usage" element={<Usage />} />
            <Route path="logs" element={<Logs />} />
          </Route>
          <Route index element={<Navigate to="/admin/dashboard" replace />} />
        </Route>
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </BrowserRouter>
  );
}

export default App;
