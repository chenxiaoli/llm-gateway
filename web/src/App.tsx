import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { getToken } from './api/client';
import Layout from './components/Layout';
import Login from './pages/Login';
import Dashboard from './pages/Dashboard';
import Keys from './pages/Keys';
import KeyDetail from './pages/KeyDetail';
import Providers from './pages/Providers';
import ProviderDetail from './pages/ProviderDetail';
import Usage from './pages/Usage';
import Logs from './pages/Logs';

function PrivateRoute({ children }: { children: React.ReactNode }) {
  const token = getToken();
  if (!token) return <Navigate to="/admin/login" replace />;
  return <>{children}</>;
}

function App() {
  return (
    <BrowserRouter basename="/admin">
      <Routes>
        <Route path="/login" element={<Login />} />
        <Route path="/" element={<PrivateRoute><Layout /></PrivateRoute>}>
          <Route index element={<Dashboard />} />
          <Route path="keys" element={<Keys />} />
          <Route path="keys/:id" element={<KeyDetail />} />
          <Route path="providers" element={<Providers />} />
          <Route path="providers/:id" element={<ProviderDetail />} />
          <Route path="usage" element={<Usage />} />
          <Route path="logs" element={<Logs />} />
        </Route>
        <Route path="*" element={<Navigate to="/admin/" replace />} />
      </Routes>
    </BrowserRouter>
  );
}

export default App;
