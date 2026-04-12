import { useState } from 'react';
import { Outlet, useNavigate, useLocation } from 'react-router-dom';
import { Layout, Menu, theme } from 'antd';
import {
  DashboardOutlined,
  KeyOutlined,
  CloudServerOutlined,
  BarChartOutlined,
  FileSearchOutlined,
  LogoutOutlined,
} from '@ant-design/icons';
import { clearToken } from '../api/client';

const { Header, Sider, Content } = Layout;

const menuItems = [
  { key: '/admin/', icon: <DashboardOutlined />, label: 'Dashboard' },
  { key: '/admin/keys', icon: <KeyOutlined />, label: 'API Keys' },
  { key: '/admin/providers', icon: <CloudServerOutlined />, label: 'Providers' },
  { key: '/admin/usage', icon: <BarChartOutlined />, label: 'Usage' },
  { key: '/admin/logs', icon: <FileSearchOutlined />, label: 'Logs' },
];

export default function AppLayout() {
  const [collapsed, setCollapsed] = useState(false);
  const navigate = useNavigate();
  const location = useLocation();
  const { token: { colorBgContainer, borderRadiusLG } } = theme.useToken();

  const handleLogout = () => {
    clearToken();
    navigate('/admin/login');
  };

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
        <Header style={{ padding: '0 16px', background: colorBgContainer, display: 'flex', justifyContent: 'flex-end' }}>
          <a onClick={handleLogout} style={{ cursor: 'pointer' }}>
            <LogoutOutlined /> Logout
          </a>
        </Header>
        <Content style={{ margin: 16 }}>
          <div style={{ padding: 24, minHeight: 360, background: colorBgContainer, borderRadius: borderRadiusLG }}>
            <Outlet />
          </div>
        </Content>
      </Layout>
    </Layout>
  );
}
