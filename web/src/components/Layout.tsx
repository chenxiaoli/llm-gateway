import { useState, useEffect } from 'react';
import { Outlet, useNavigate, useLocation } from 'react-router-dom';
import {
  DashboardOutlined,
  KeyOutlined,
  CloudServerOutlined,
  BarChartOutlined,
  FileSearchOutlined,
  LogoutOutlined,
  SettingOutlined,
  TeamOutlined,
  MenuFoldOutlined,
  MenuUnfoldOutlined,
} from '@ant-design/icons';
import { useAuth } from '../contexts/AuthContext';
import { apiClient } from '../api/client';

const consoleItems = [
  { key: '/console/dashboard', icon: <DashboardOutlined />, label: 'Dashboard' },
  { key: '/console/keys', icon: <KeyOutlined />, label: 'API Keys' },
  { key: '/console/usage', icon: <BarChartOutlined />, label: 'Usage' },
];

const adminItems = [
  { key: '/console/providers', icon: <CloudServerOutlined />, label: 'Providers' },
  { key: '/console/users', icon: <TeamOutlined />, label: 'Users' },
  { key: '/console/settings', icon: <SettingOutlined />, label: 'Settings' },
  { key: '/console/logs', icon: <FileSearchOutlined />, label: 'Logs' },
];

export default function AppLayout() {
  const [collapsed, setCollapsed] = useState(false);
  const [version, setVersion] = useState('');
  const navigate = useNavigate();
  const location = useLocation();
  const { user, logout } = useAuth();
  const isAdmin = user?.role === 'admin';

  useEffect(() => {
    apiClient.get<{ version: string }>('/version').then((r) => setVersion(r.data.version));
  }, []);

  return (
    <div style={{ minHeight: '100vh' }}>
      {/* Sidebar */}
      <aside className={`console-sidebar ${collapsed ? 'collapsed' : ''}`}>
        <div className="sidebar-logo" onClick={() => navigate('/console/dashboard')}>
          <div className="sidebar-logo-icon">GW</div>
          <span className="sidebar-logo-text">LLM Gateway</span>
        </div>

        <nav className="sidebar-nav">
          <div className="sidebar-nav-group-label">Console</div>
          {consoleItems.map((item) => (
            <div
              key={item.key}
              className={`sidebar-nav-item ${location.pathname === item.key ? 'active' : ''}`}
              onClick={() => navigate(item.key)}
            >
              <span className="nav-icon">{item.icon}</span>
              <span className="nav-label">{item.label}</span>
            </div>
          ))}

          {isAdmin && (
            <>
              <div className="sidebar-nav-group-label">Admin</div>
              {adminItems.map((item) => (
                <div
                  key={item.key}
                  className={`sidebar-nav-item ${location.pathname === item.key ? 'active' : ''}`}
                  onClick={() => navigate(item.key)}
                >
                  <span className="nav-icon">{item.icon}</span>
                  <span className="nav-label">{item.label}</span>
                </div>
              ))}
            </>
          )}
        </nav>

        <div className="sidebar-footer">
          <button className="sidebar-collapse-btn" onClick={() => setCollapsed(!collapsed)}>
            {collapsed ? <MenuUnfoldOutlined /> : <MenuFoldOutlined />}
          </button>
        </div>
      </aside>

      {/* Main content area */}
      <div className={`console-main ${collapsed ? 'sidebar-collapsed' : ''}`}>
        <header className="console-header">
          <div className="header-user">
            <div className="header-avatar">{user?.username?.charAt(0).toUpperCase()}</div>
            <span>{user?.username}</span>
          </div>
          <button className="header-logout" onClick={logout}>
            <LogoutOutlined /> Logout
          </button>
        </header>

        <main className="console-content">
          <div className="page-enter">
            <Outlet />
          </div>
        </main>

        <footer className="console-footer">
          LLM Gateway{version ? ` ${version}` : ''}
        </footer>
      </div>
    </div>
  );
}
