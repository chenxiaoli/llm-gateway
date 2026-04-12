import { useState } from 'react';
import { useNavigate, Link } from 'react-router-dom';
import { Form, Input, Button, Card, Typography, message, Alert } from 'antd';
import { useAuth } from '../contexts/AuthContext';
import { useQuery } from '@tanstack/react-query';
import { getAuthConfig } from '../api/auth';

const { Title, Text } = Typography;

export default function Register() {
  const [loading, setLoading] = useState(false);
  const navigate = useNavigate();
  const { register } = useAuth();

  const { data: authConfig } = useQuery({
    queryKey: ['authConfig'],
    queryFn: getAuthConfig,
    retry: false,
  });

  const registrationDisabled = authConfig !== undefined && !authConfig.allow_registration;

  const onFinish = async (values: { username: string; password: string }) => {
    setLoading(true);
    try {
      await register(values);
      navigate('/console/dashboard');
    } catch {
      message.error('Registration failed');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', minHeight: '100vh' }}>
      <Card style={{ width: 400 }}>
        <Title level={3} style={{ textAlign: 'center' }}>Create Account</Title>
        {registrationDisabled && (
          <Alert message="Registration is currently disabled" type="warning" style={{ marginBottom: 16 }} />
        )}
        <Form onFinish={onFinish} layout="vertical">
          <Form.Item name="username" label="Username" rules={[
            { required: true, message: 'Enter a username' },
            { min: 3, message: 'Username must be at least 3 characters' },
          ]}>
            <Input placeholder="Username" disabled={registrationDisabled} />
          </Form.Item>
          <Form.Item name="password" label="Password" rules={[
            { required: true, message: 'Enter a password' },
            { min: 6, message: 'Password must be at least 6 characters' },
          ]}>
            <Input.Password placeholder="Password" disabled={registrationDisabled} />
          </Form.Item>
          <Form.Item name="confirm" label="Confirm Password" dependencies={['password']} rules={[
            { required: true, message: 'Confirm your password' },
            ({ getFieldValue }) => ({
              validator(_, value) {
                if (!value || getFieldValue('password') === value) return Promise.resolve();
                return Promise.reject(new Error('Passwords do not match'));
              },
            }),
          ]}>
            <Input.Password placeholder="Confirm password" disabled={registrationDisabled} />
          </Form.Item>
          <Form.Item>
            <Button type="primary" htmlType="submit" loading={loading} block disabled={registrationDisabled}>
              Register
            </Button>
          </Form.Item>
        </Form>
        <Text style={{ display: 'block', textAlign: 'center' }}>
          <Link to="/console/login">Already have an account? Login</Link>
        </Text>
      </Card>
    </div>
  );
}
