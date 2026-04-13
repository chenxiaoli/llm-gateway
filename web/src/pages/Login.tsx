import { useState } from 'react';
import { useNavigate, Link } from 'react-router-dom';
import { Form, Input, Button, Typography, message } from 'antd';
import { useAuth } from '../contexts/AuthContext';
import { useQuery } from '@tanstack/react-query';
import { getAuthConfig } from '../api/auth';

const { Text } = Typography;

export default function Login() {
  const [loading, setLoading] = useState(false);
  const navigate = useNavigate();
  const { login } = useAuth();

  const { data: authConfig } = useQuery({
    queryKey: ['authConfig'],
    queryFn: getAuthConfig,
    retry: false,
  });

  const onFinish = async (values: { username: string; password: string }) => {
    setLoading(true);
    try {
      await login(values);
      navigate('/console/dashboard');
    } catch {
      message.error('Invalid username or password');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="login-page">
      <div className="login-bg" />
      <div className="login-grid" />
      <div className="login-card">
        <div className="login-logo">
          <div className="login-logo-icon">GW</div>
          <span className="login-logo-text">LLM Gateway</span>
        </div>
        <Form onFinish={onFinish} layout="vertical">
          <Form.Item name="username" label="Username" rules={[{ required: true, message: 'Enter your username' }]}>
            <Input placeholder="Username" size="large" />
          </Form.Item>
          <Form.Item name="password" label="Password" rules={[{ required: true, message: 'Enter your password' }]}>
            <Input.Password placeholder="Password" size="large" />
          </Form.Item>
          <Form.Item style={{ marginBottom: authConfig?.allow_registration ? 20 : 0 }}>
            <Button type="primary" htmlType="submit" loading={loading} block size="large">
              Sign In
            </Button>
          </Form.Item>
        </Form>
        {authConfig?.allow_registration && (
          <div style={{ textAlign: 'center' }}>
            <Text style={{ color: 'var(--text-secondary)', fontSize: 14 }}>
              Don't have an account? <Link to="/console/register" style={{ color: 'var(--accent)' }}>Create one</Link>
            </Text>
          </div>
        )}
      </div>
    </div>
  );
}
