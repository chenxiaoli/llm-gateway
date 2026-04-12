import { Typography, Row, Col, Table, Tag } from 'antd';
import { DollarOutlined, MessageOutlined, ApiOutlined } from '@ant-design/icons';
import StatCard from '../components/StatCard';
import { useLogs } from '../hooks/useLogs';
import { useUsage } from '../hooks/useUsage';
import dayjs from 'dayjs';

const { Title } = Typography;

export default function Dashboard() {
  const today = dayjs().startOf('day').toISOString();
  const monthStart = dayjs().startOf('month').toISOString();

  const { data: allUsage } = useUsage({});
  const { data: todayUsage } = useUsage({ since: today });
  const { data: monthUsage } = useUsage({ since: monthStart });
  const { data: recentLogs } = useLogs({});

  const todayItems = todayUsage?.items ?? [];
  const monthItems = monthUsage?.items ?? [];
  const allItems = allUsage?.items ?? [];

  const todayRequests = todayItems.length;
  const todayCost = todayItems.reduce((sum, r) => sum + r.cost, 0);
  const monthCost = monthItems.reduce((sum, r) => sum + r.cost, 0);
  const totalModels = new Set(allItems.map(r => r.model_name)).size;

  const logColumns = [
    { title: 'Time', dataIndex: 'created_at', key: 'created_at', width: 180,
      render: (v: string) => new Date(v).toLocaleString(),
    },
    { title: 'Model', dataIndex: 'model_name', key: 'model_name' },
    { title: 'Protocol', dataIndex: 'protocol', key: 'protocol',
      render: (v: string) => <Tag color={v === 'openai' ? 'blue' : 'purple'}>{v}</Tag>,
    },
    { title: 'Status', dataIndex: 'status_code', key: 'status_code',
      render: (v: number) => <Tag color={v < 400 ? 'green' : 'red'}>{v}</Tag>,
    },
    { title: 'Tokens', key: 'tokens',
      render: (_: unknown, r: { input_tokens: number | null; output_tokens: number | null }) =>
        `${r.input_tokens ?? 0} + ${r.output_tokens ?? 0}`,
    },
    { title: 'Cost', dataIndex: 'cost', key: 'cost',
      render: (v: number) => `$${v.toFixed(6)}`,
    },
    { title: 'Latency', dataIndex: 'latency_ms', key: 'latency_ms',
      render: (v: number) => `${v}ms`,
    },
  ];

  return (
    <div>
      <Title level={4}>Dashboard</Title>
      <Row gutter={16} style={{ marginBottom: 24 }}>
        <Col span={6}>
          <StatCard title="Today's Requests" value={todayRequests} prefix={<MessageOutlined />} />
        </Col>
        <Col span={6}>
          <StatCard title="Today's Cost" value={`$${todayCost.toFixed(4)}`} prefix={<DollarOutlined />} />
        </Col>
        <Col span={6}>
          <StatCard title="Monthly Cost" value={`$${monthCost.toFixed(2)}`} prefix={<DollarOutlined />} />
        </Col>
        <Col span={6}>
          <StatCard title="Active Models" value={totalModels} prefix={<ApiOutlined />} />
        </Col>
      </Row>

      <Title level={5}>Recent Requests</Title>
      <Table
        dataSource={recentLogs?.items}
        columns={logColumns}
        rowKey="id"
        size="small"
        pagination={false}
        scroll={{ x: 700 }}
      />
    </div>
  );
}
