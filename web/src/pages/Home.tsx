import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { Button, Typography, Space, Row, Col, Divider } from 'antd';
import {
  ApiOutlined,
  KeyOutlined,
  CloudServerOutlined,
  SafetyCertificateOutlined,
  DashboardOutlined,
  ThunderboltOutlined,
} from '@ant-design/icons';
import { apiClient } from '../api/client';

const { Title, Paragraph, Text } = Typography;

const features = [
  {
    icon: <ApiOutlined style={{ fontSize: 28, color: '#06d6a0' }} />,
    title: 'Dual Protocol',
    desc: 'OpenAI and Anthropic compatible API endpoints. Drop-in replacement for your existing SDK.',
  },
  {
    icon: <CloudServerOutlined style={{ fontSize: 28, color: '#3b82f6' }} />,
    title: 'Multi-Provider',
    desc: 'Route to multiple upstream LLM providers. Balance load, avoid vendor lock-in.',
  },
  {
    icon: <KeyOutlined style={{ fontSize: 28, color: '#f59e0b' }} />,
    title: 'API Key Management',
    desc: 'Create, rotate, and revoke keys. Per-key rate limits and monthly budgets.',
  },
  {
    icon: <ThunderboltOutlined style={{ fontSize: 28, color: '#ef4444' }} />,
    title: 'Rate Limiting',
    desc: 'Per-key and per-model RPM/TPM limits. In-memory sliding window enforcement.',
  },
  {
    icon: <SafetyCertificateOutlined style={{ fontSize: 28, color: '#a855f7' }} />,
    title: 'Audit Logging',
    desc: 'Full request logging with latency, tokens, and cost tracking. Retention policies.',
  },
  {
    icon: <DashboardOutlined style={{ fontSize: 28, color: '#06b6d4' }} />,
    title: 'Web Dashboard',
    desc: 'Management UI for keys, providers, models, usage analytics, and audit logs.',
  },
];

export default function Home() {
  const navigate = useNavigate();
  const [version, setVersion] = useState('');

  useEffect(() => {
    apiClient.get<{ version: string }>('/version').then((r) => setVersion(r.data.version));
  }, []);

  return (
    <div style={{ minHeight: '100vh', background: 'var(--bg-base)', position: 'relative' }}>
      {/* Hero */}
      <div style={{
        maxWidth: 900, margin: '0 auto', padding: '80px 24px 60px',
        textAlign: 'center',
      }}>
        <Title style={{ fontSize: 48, fontWeight: 800, marginBottom: 16, fontFamily: 'var(--font-display)', color: 'var(--text-primary)' }}>
          LLM Gateway
        </Title>
        <Paragraph style={{ fontSize: 20, color: 'var(--text-secondary)', maxWidth: 600, margin: '0 auto 32px' }}>
          A unified API gateway for LLM providers. Manage keys, enforce rate limits,
          track usage, and proxy requests to OpenAI and Anthropic from a single endpoint.
        </Paragraph>
        <Space size="large">
          <Button type="primary" size="large" onClick={() => navigate('/console/login')}>
            Go to Dashboard
          </Button>
          <Button size="large" onClick={() => window.open('https://github.com/chenxiaoli/llm-gateway', '_blank')}>
            GitHub
          </Button>
        </Space>
      </div>

      {/* Features */}
      <div style={{ maxWidth: 1000, margin: '0 auto', padding: '0 24px 80px' }}>
        <Divider plain>
          <Text strong style={{ fontSize: 13, color: 'var(--text-muted)', textTransform: 'uppercase', letterSpacing: '0.08em' }}>Features</Text>
        </Divider>
        <Row gutter={[20, 20]}>
          {features.map((f) => (
            <Col xs={24} sm={12} lg={8} key={f.title}>
              <div style={{
                height: '100%', textAlign: 'center', padding: '28px 20px',
                background: 'var(--bg-card)', border: '1px solid var(--border-subtle)',
                borderRadius: 12,
              }}>
                <div style={{ marginBottom: 16 }}>{f.icon}</div>
                <Title level={5} style={{ fontFamily: 'var(--font-display)', marginBottom: 8 }}>{f.title}</Title>
                <Paragraph type="secondary" style={{ color: 'var(--text-muted)', fontSize: 14 }}>{f.desc}</Paragraph>
              </div>
            </Col>
          ))}
        </Row>
      </div>

      {/* Quick start */}
      <div style={{
        maxWidth: 700, margin: '0 auto', padding: '0 24px 80px',
        textAlign: 'center',
      }}>
        <Divider plain>
          <Text strong style={{ fontSize: 13, color: 'var(--text-muted)', textTransform: 'uppercase', letterSpacing: '0.08em' }}>Quick Start</Text>
        </Divider>
        <div style={{
          padding: '24px 28px', background: 'var(--bg-card)',
          border: '1px solid var(--border-subtle)', borderRadius: 12,
        }}>
          <div style={{ textAlign: 'left', fontFamily: 'var(--font-mono)', fontSize: 14, lineHeight: 2 }}>
            <div><Text type="secondary"># Create an API key via the management dashboard</Text></div>
            <div><Text type="secondary"># Then use it as a drop-in replacement:</Text></div>
            <br />
            <div style={{ color: 'var(--text-primary)' }}>curl -X POST http://localhost:8080/v1/chat/completions \</div>
            <div style={{ color: 'var(--text-primary)' }}>&nbsp;&nbsp;-H "Authorization: Bearer <span style={{color:'#06d6a0'}}>your-api-key</span>" \</div>
            <div style={{ color: 'var(--text-primary)' }}>&nbsp;&nbsp;-H "Content-Type: application/json" \</div>
            <div style={{ color: 'var(--text-primary)' }}>&nbsp;&nbsp;-d {"'{{\"model\": \"gpt-4\", \"messages\": [{{\"role\": \"user\", \"content\": \"Hello\"}}]}}'"}</div>
          </div>
        </div>
      </div>

      {/* Footer */}
      <div style={{ textAlign: 'center', padding: '24px 0', color: 'var(--text-muted)' }}>
        <Text type="secondary">LLM Gateway{version ? ` ${version}` : ''} &mdash; Open Source</Text>
      </div>
    </div>
  );
}
