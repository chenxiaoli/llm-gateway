import { useState } from 'react';
import { Typography, DatePicker, Select, Table, Card, Row, Col, Statistic } from 'antd';
import { DollarOutlined, MessageOutlined } from '@ant-design/icons';
import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from 'recharts';
import { useUsage } from '../hooks/useUsage';
import { useKeys } from '../hooks/useKeys';
import { Dayjs } from 'dayjs';

const { RangePicker } = DatePicker;
const { Title } = Typography;

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
      render: (v: string) => new Date(v).toLocaleString(),
    },
    { title: 'Key ID', dataIndex: 'key_id', key: 'key_id',
      render: (v: string) => v.substring(0, 8) + '...',
    },
    { title: 'Model', dataIndex: 'model_name', key: 'model_name' },
    { title: 'Protocol', dataIndex: 'protocol', key: 'protocol' },
    { title: 'Input Tokens', dataIndex: 'input_tokens', key: 'input_tokens', render: (v: number | null) => v ?? '-' },
    { title: 'Output Tokens', dataIndex: 'output_tokens', key: 'output_tokens', render: (v: number | null) => v ?? '-' },
    { title: 'Cost', dataIndex: 'cost', key: 'cost', render: (v: number) => `$${v.toFixed(6)}` },
  ];

  return (
    <div>
      <Title level={4}>Usage</Title>

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

      <Row gutter={16} style={{ marginBottom: 16 }}>
        <Col span={6}><Card><Statistic title="Total Cost" value={`$${totalCost.toFixed(4)}`} prefix={<DollarOutlined />} /></Card></Col>
        <Col span={6}><Card><Statistic title="Total Requests" value={totalRequests} prefix={<MessageOutlined />} /></Card></Col>
        <Col span={6}><Card><Statistic title="Input Tokens" value={totalInputTokens} /></Card></Col>
        <Col span={6}><Card><Statistic title="Output Tokens" value={totalOutputTokens} /></Card></Col>
      </Row>

      {chartData.length > 0 && (
        <Card title="Cost by Model" style={{ marginBottom: 16 }}>
          <ResponsiveContainer width="100%" height={300}>
            <BarChart data={chartData}>
              <CartesianGrid strokeDasharray="3 3" />
              <XAxis dataKey="model" />
              <YAxis />
              <Tooltip formatter={(value: number) => `$${value.toFixed(4)}`} />
              <Bar dataKey="cost" fill="#1890ff" name="Cost ($)" />
            </BarChart>
          </ResponsiveContainer>
        </Card>
      )}

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
  );
}
