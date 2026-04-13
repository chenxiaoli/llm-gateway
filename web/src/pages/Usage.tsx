import { useState } from 'react';
import { DatePicker, Select, Table, Card, Row, Col } from 'antd';
import { DollarOutlined, MessageOutlined } from '@ant-design/icons';
import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from 'recharts';
import { useUsage } from '../hooks/useUsage';
import { useKeys } from '../hooks/useKeys';
import { Dayjs } from 'dayjs';

const { RangePicker } = DatePicker;

export default function Usage() {
  const [dateRange, setDateRange] = useState<[Dayjs | null, Dayjs | null] | null>(null);
  const [keyFilter, setKeyFilter] = useState<string | undefined>(undefined);
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(20);

  const since = dateRange?.[0]?.toISOString();
  const until = dateRange?.[1]?.toISOString();

  const { data, isLoading } = useUsage({ since, until, key_id: keyFilter }, page, pageSize);
  const { data: keys } = useKeys();

  const usageItems = data?.items ?? [];
  const totalCost = usageItems.reduce((sum, r) => sum + r.cost, 0);
  const totalRequests = usageItems.length;
  const totalInputTokens = usageItems.reduce((sum, r) => sum + (r.input_tokens ?? 0), 0);
  const totalOutputTokens = usageItems.reduce((sum, r) => sum + (r.output_tokens ?? 0), 0);

  const byModel: Record<string, { model: string; requests: number; cost: number }> = {};
  usageItems.forEach((r) => {
    if (!byModel[r.model_name]) {
      byModel[r.model_name] = { model: r.model_name, requests: 0, cost: 0 };
    }
    byModel[r.model_name].requests += 1;
    byModel[r.model_name].cost += r.cost;
  });
  const chartData = Object.values(byModel).sort((a, b) => b.cost - a.cost);

  const columns = [
    { title: 'Time', dataIndex: 'created_at', key: 'created_at',
      render: (v: string) => <span className="mono">{new Date(v).toLocaleString()}</span>,
    },
    { title: 'Key ID', dataIndex: 'key_id', key: 'key_id',
      render: (v: string) => <span className="mono">{v.substring(0, 8)}...</span>,
    },
    { title: 'Model', dataIndex: 'model_name', key: 'model_name',
      render: (v: string) => <span className="mono">{v}</span>,
    },
    { title: 'Protocol', dataIndex: 'protocol', key: 'protocol' },
    { title: 'Input Tokens', dataIndex: 'input_tokens', key: 'input_tokens', render: (v: number | null) => <span className="mono">{v ?? '-'}</span> },
    { title: 'Output Tokens', dataIndex: 'output_tokens', key: 'output_tokens', render: (v: number | null) => <span className="mono">{v ?? '-'}</span> },
    { title: 'Cost', dataIndex: 'cost', key: 'cost', render: (v: number) => <span className="mono">${v.toFixed(6)}</span> },
  ];

  return (
    <div>
      <div className="page-header">
        <h1 className="page-title">Usage</h1>
      </div>

      <Card style={{ marginBottom: 16 }}>
        <Row gutter={16} align="middle">
          <Col>
            <RangePicker
              onChange={(dates) => setDateRange(dates as [Dayjs | null, Dayjs | null] | null)}
            />
          </Col>
          <Col>
            <Select
              placeholder="Filter by API Key"
              allowClear
              style={{ width: 200 }}
              onChange={(v) => setKeyFilter(v)}
              options={keys?.items?.map(k => ({ value: k.id, label: k.name })) ?? []}
            />
          </Col>
        </Row>
      </Card>

      <div className="stat-cards-grid">
        <div className="stat-card" style={{ '--card-accent': '#06d6a0' } as React.CSSProperties}>
          <div className="stat-card-label">Total Cost</div>
          <div className="stat-card-value">${totalCost.toFixed(4)}</div>
          <div className="stat-card-icon"><DollarOutlined /></div>
        </div>
        <div className="stat-card" style={{ '--card-accent': '#3b82f6' } as React.CSSProperties}>
          <div className="stat-card-label">Total Requests</div>
          <div className="stat-card-value">{totalRequests.toLocaleString()}</div>
          <div className="stat-card-icon"><MessageOutlined /></div>
        </div>
        <div className="stat-card" style={{ '--card-accent': '#f59e0b' } as React.CSSProperties}>
          <div className="stat-card-label">Input Tokens</div>
          <div className="stat-card-value">{totalInputTokens.toLocaleString()}</div>
        </div>
        <div className="stat-card" style={{ '--card-accent': '#a855f7' } as React.CSSProperties}>
          <div className="stat-card-label">Output Tokens</div>
          <div className="stat-card-value">{totalOutputTokens.toLocaleString()}</div>
        </div>
      </div>

      {chartData.length > 0 && (
        <Card title={<h3 className="page-title" style={{ fontSize: 16, margin: 0 }}>Cost by Model</h3>} style={{ marginBottom: 16 }}>
          <ResponsiveContainer width="100%" height={300}>
            <BarChart data={chartData}>
              <CartesianGrid strokeDasharray="3 3" stroke="#27272a" />
              <XAxis dataKey="model" tick={{ fill: '#71717a', fontSize: 12 }} />
              <YAxis tick={{ fill: '#71717a', fontSize: 12 }} />
              <Tooltip
                contentStyle={{ background: '#1e1e22', border: '1px solid #3f3f46', borderRadius: 8 }}
                labelStyle={{ color: '#fafafa' }}
                formatter={(value: number) => [`$${value.toFixed(4)}`, 'Cost']}
              />
              <Bar dataKey="cost" fill="#06d6a0" name="Cost ($)" radius={[4, 4, 0, 0]} />
            </BarChart>
          </ResponsiveContainer>
        </Card>
      )}

      <div className="console-table">
        <Table
          dataSource={usageItems}
          columns={columns}
          rowKey="id"
          loading={isLoading}
          size="small"
          scroll={{ x: 800 }}
          pagination={{
            current: page,
            pageSize,
            total: data?.total ?? 0,
            onChange: (p, ps) => { setPage(p); setPageSize(ps); },
            showSizeChanger: true,
            showTotal: (total) => `Total ${total}`,
          }}
        />
      </div>
    </div>
  );
}
