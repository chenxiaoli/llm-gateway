import { useState, useEffect } from 'react';
import { Outlet, useNavigate, useLocation } from 'react-router-dom';
import { Layout, Menu, theme, Space } from 'antd';
import {
  DashboardOutlined,
  KeyOutlined,
  CloudServerOutlined,
  BarChartOutlined,
  FileSearchOutlined,
  LogoutOutlined,
  UserOutlined,
  SettingOutlined,
  TeamOutlined,
} from '@ant-design/icons';
import { useAuth } from '../contexts/AuthContext';
import { apiClient } from '../api/client';

const { Header, Sider, Content, Footer } = Layout;

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
  const { token: { colorBgContainer, borderRadiusLG } } = theme.useToken();

  useEffect(() => {
    apiClient.get<{ version: string }>('/version').then((r) => setVersion(r.data.version));
  }, []);

  const isAdmin = user?.role === 'admin';

  const menuItems = [
    { key: 'console', label: 'Console', type: 'group' as const, children: consoleItems },
    ...(isAdmin ? [{ key: 'admin', label: 'Admin', type: 'group' as const, children: adminItems }] : []),
  ];

  return (
    <Layout style={{ minHeight: '100vh' }}>
      <Sider collapsible collapsed={collapsed} onCollapse={setCollapsed}>
        <div style={{ height: 32, margin: 16, textAlign: 'center', color: '#fff', fontSize: collapsed ? 14 : 16, fontWeight: 'bold' }}>
          {collapsed ? 'GW' : 'LLM Gateway'}
        </div>
        <Menu
          theme="dark"
          mode="inline"
          selectedKeys={[location.pathname]}
          items={menuItems}
          onClick={({ key }) => navigate(key)}
        />
      </Sider>
      <Layout>
        <Header style={{ padding: '0 16px', background: colorBgContainer, display: 'flex', justifyContent: 'flex-end', alignItems: 'center' }}>
          <Space size="middle">
            <span><UserOutlined /> {user?.username}</span>
            <a onClick={logout} style={{ cursor: 'pointer' }}>
              <LogoutOutlined /> Logout
            </a>
          </Space>
        </Header>
        <Content style={{ margin: 16 }}>
          <div style={{ padding: 24, minHeight: 360, background: colorBgContainer, borderRadius: borderRadiusLG }}>
            <Outlet />
          </div>
        </Content>
        <Footer style={{ textAlign: 'center', color: '#999' }}>
          LLM Gateway{version ? ` ${version}` : ''}
        </Footer>
      </Layout>
    </Layout>
  );
}
