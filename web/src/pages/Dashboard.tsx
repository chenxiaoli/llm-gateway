import { Table, Tag } from 'antd';
import { DollarOutlined, MessageOutlined, ApiOutlined } from '@ant-design/icons';
import { useLogs } from '../hooks/useLogs';
import { useUsage } from '../hooks/useUsage';
import dayjs from 'dayjs';

export default function Dashboard() {
  const today = dayjs().startOf('day').toISOString();
  const monthStart = dayjs().startOf('month').toISOString();

  const { data: allUsage } = useUsage({}, 1, 99999);
  const { data: todayUsage } = useUsage({ since: today }, 1, 99999);
  const { data: monthUsage } = useUsage({ since: monthStart }, 1, 99999);
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
      render: (v: string) => <span className="mono">{new Date(v).toLocaleString()}</span>,
    },
    { title: 'Model', dataIndex: 'model_name', key: 'model_name',
      render: (v: string) => <span className="mono">{v}</span>,
    },
    { title: 'Protocol', dataIndex: 'protocol', key: 'protocol',
      render: (v: string) => <Tag color={v === 'openai' ? '#3b82f6' : '#a855f7'}>{v}</Tag>,
    },
    { title: 'Status', dataIndex: 'status_code', key: 'status_code',
      render: (v: number) => <Tag color={v < 400 ? '#06d6a0' : v < 500 ? '#f59e0b' : '#ef4444'}>{v}</Tag>,
    },
    { title: 'Tokens', key: 'tokens',
      render: (_: unknown, r: { input_tokens: number | null; output_tokens: number | null }) =>
        <span className="mono">{r.input_tokens ?? 0} + {r.output_tokens ?? 0}</span>,
    },
    { title: 'Cost', dataIndex: 'cost', key: 'cost',
      render: (v: number) => <span className="mono">${v.toFixed(6)}</span>,
    },
    { title: 'Latency', dataIndex: 'latency_ms', key: 'latency_ms',
      render: (v: number) => <span className="mono">{v}ms</span>,
    },
  ];

  return (
    <div>
      <div className="page-header">
        <h1 className="page-title">Dashboard</h1>
      </div>

      <div className="stat-cards-grid">
        <div className="stat-card" style={{ '--card-accent': '#06d6a0' } as React.CSSProperties}>
          <div className="stat-card-label">Today's Requests</div>
          <div className="stat-card-value">{todayRequests.toLocaleString()}</div>
          <div className="stat-card-icon"><MessageOutlined /></div>
        </div>
        <div className="stat-card" style={{ '--card-accent': '#f59e0b' } as React.CSSProperties}>
          <div className="stat-card-label">Today's Cost</div>
          <div className="stat-card-value">${todayCost.toFixed(4)}</div>
          <div className="stat-card-icon"><DollarOutlined /></div>
        </div>
        <div className="stat-card" style={{ '--card-accent': '#3b82f6' } as React.CSSProperties}>
          <div className="stat-card-label">Monthly Cost</div>
          <div className="stat-card-value">${monthCost.toFixed(2)}</div>
          <div className="stat-card-icon"><DollarOutlined /></div>
        </div>
        <div className="stat-card" style={{ '--card-accent': '#a855f7' } as React.CSSProperties}>
          <div className="stat-card-label">Active Models</div>
          <div className="stat-card-value">{totalModels}</div>
          <div className="stat-card-icon"><ApiOutlined /></div>
        </div>
      </div>

      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 16 }}>
        <h2 className="page-title" style={{ fontSize: 16, fontWeight: 600 }}>Recent Requests</h2>
      </div>

      <div className="console-table">
        <Table
          dataSource={recentLogs?.items}
          columns={logColumns}
          rowKey="id"
          size="small"
          pagination={false}
          scroll={{ x: 700 }}
        />
      </div>
    </div>
  );
}
