import { useNavigate } from 'react-router-dom';
import { Button, Typography, Space, Card, Row, Col, Divider } from 'antd';
import {
  ApiOutlined,
  KeyOutlined,
  CloudServerOutlined,
  SafetyCertificateOutlined,
  DashboardOutlined,
  ThunderboltOutlined,
} from '@ant-design/icons';

const { Title, Paragraph, Text } = Typography;

const features = [
  {
    icon: <ApiOutlined style={{ fontSize: 32, color: '#1677ff' }} />,
    title: 'Dual Protocol',
    desc: 'OpenAI and Anthropic compatible API endpoints. Drop-in replacement for your existing SDK.',
  },
  {
    icon: <CloudServerOutlined style={{ fontSize: 32, color: '#52c41a' }} />,
    title: 'Multi-Provider',
    desc: 'Route to multiple upstream LLM providers. Balance load, avoid vendor lock-in.',
  },
  {
    icon: <KeyOutlined style={{ fontSize: 32, color: '#faad14' }} />,
    title: 'API Key Management',
    desc: 'Create, rotate, and revoke keys. Per-key rate limits and monthly budgets.',
  },
  {
    icon: <ThunderboltOutlined style={{ fontSize: 32, color: '#eb2f96' }} />,
    title: 'Rate Limiting',
    desc: 'Per-key and per-model RPM/TPM limits. In-memory sliding window enforcement.',
  },
  {
    icon: <SafetyCertificateOutlined style={{ fontSize: 32, color: '#722ed1' }} />,
    title: 'Audit Logging',
    desc: 'Full request logging with latency, tokens, and cost tracking. Retention policies.',
  },
  {
    icon: <DashboardOutlined style={{ fontSize: 32, color: '#13c2c2' }} />,
    title: 'Web Dashboard',
    desc: 'Management UI for keys, providers, models, usage analytics, and audit logs.',
  },
];

export default function Home() {
  const navigate = useNavigate();

  return (
    <div style={{ minHeight: '100vh', background: '#f5f5f5' }}>
      {/* Hero */}
      <div style={{
        maxWidth: 900, margin: '0 auto', padding: '80px 24px 60px',
        textAlign: 'center',
      }}>
        <Title style={{ fontSize: 48, fontWeight: 700, marginBottom: 16 }}>
          LLM Gateway
        </Title>
        <Paragraph style={{ fontSize: 20, color: '#666', maxWidth: 600, margin: '0 auto 32px' }}>
          A unified API gateway for LLM providers. Manage keys, enforce rate limits,
          track usage, and proxy requests to OpenAI and Anthropic from a single endpoint.
        </Paragraph>
        <Space size="large">
          <Button type="primary" size="large" onClick={() => navigate('/admin/login')}>
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
          <Text strong style={{ fontSize: 16, color: '#999' }}>FEATURES</Text>
        </Divider>
        <Row gutter={[24, 24]}>
          {features.map((f) => (
            <Col xs={24} sm={12} lg={8} key={f.title}>
              <Card style={{ height: '100%', textAlign: 'center' }} hoverable>
                <div style={{ marginBottom: 16 }}>{f.icon}</div>
                <Title level={5}>{f.title}</Title>
                <Paragraph type="secondary">{f.desc}</Paragraph>
              </Card>
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
          <Text strong style={{ fontSize: 16, color: '#999' }}>QUICK START</Text>
        </Divider>
        <Card>
          <div style={{ textAlign: 'left', fontFamily: 'monospace', fontSize: 14, lineHeight: 2 }}>
            <div><Text type="secondary"># Create an API key via the management dashboard</Text></div>
            <div><Text type="secondary"># Then use it as a drop-in replacement:</Text></div>
            <br />
            <div>curl -X POST http://localhost:8080/v1/chat/completions \</div>
            <div>&nbsp;&nbsp;-H "Authorization: Bearer <span style={{color:'#1677ff'}}>your-api-key</span>" \</div>
            <div>&nbsp;&nbsp;-H "Content-Type: application/json" \</div>
            <div>&nbsp;&nbsp;-d {"'{{\"model\": \"gpt-4\", \"messages\": [{{\"role\": \"user\", \"content\": \"Hello\"}}]}}'"}</div>
          </div>
        </Card>
      </div>

      {/* Footer */}
      <div style={{ textAlign: 'center', padding: '24px 0', color: '#999' }}>
        <Text type="secondary">LLM Gateway &mdash; Open Source</Text>
      </div>
    </div>
  );
}
