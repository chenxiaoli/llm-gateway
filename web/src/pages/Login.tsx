import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Form, Input, Button, Card, Typography, message } from 'antd';
import { setToken } from '../api/client';
import { apiClient } from '../api/client';

const { Title } = Typography;

export default function Login() {
  const [loading, setLoading] = useState(false);
  const navigate = useNavigate();

  const onFinish = async (values: { token: string }) => {
    setLoading(true);
    setToken(values.token);
    try {
      await apiClient.get('/keys');
      navigate('/admin/');
    } catch {
      message.error('Invalid admin token');
      setToken('');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', minHeight: '100vh' }}>
      <Card style={{ width: 400 }}>
        <Title level={3} style={{ textAlign: 'center' }}>LLM Gateway</Title>
        <Form onFinish={onFinish} layout="vertical">
          <Form.Item name="token" label="Admin Token" rules={[{ required: true }]}>
            <Input.Password placeholder="Enter admin token" />
          </Form.Item>
          <Form.Item>
            <Button type="primary" htmlType="submit" loading={loading} block>
              Login
            </Button>
          </Form.Item>
        </Form>
      </Card>
    </div>
  );
}
