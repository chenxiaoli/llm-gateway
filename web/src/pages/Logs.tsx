import { useState } from 'react';
import { Typography, DatePicker, Select, Table, Card, Row, Col, Tag, Drawer } from 'antd';
import { useLogs } from '../hooks/useLogs';
import { useKeys } from '../hooks/useKeys';
import JsonViewer from '../components/JsonViewer';
import { Dayjs } from 'dayjs';
import type { AuditLog } from '../types';

const { RangePicker } = DatePicker;
const { Title } = Typography;

export default function Logs() {
  const [dateRange, setDateRange] = useState<[Dayjs | null, Dayjs | null] | null>(null);
  const [keyFilter, setKeyFilter] = useState<string | undefined>(undefined);
  const [selectedLog, setSelectedLog] = useState<AuditLog | null>(null);
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(20);

  const since = dateRange?.[0]?.toISOString();
  const until = dateRange?.[1]?.toISOString();

  const { data, isLoading } = useLogs({ since, until, key_id: keyFilter }, page, pageSize);
  const { data: keys } = useKeys();

  const columns = [
    { title: 'Time', dataIndex: 'created_at', key: 'created_at', width: 180,
      render: (v: string) => <span className="mono">{new Date(v).toLocaleString()}</span>,
    },
    { title: 'Model', dataIndex: 'model_name', key: 'model_name', width: 150,
      render: (v: string) => <span className="mono">{v}</span>,
    },
    { title: 'Protocol', dataIndex: 'protocol', key: 'protocol', width: 100,
      render: (v: string) => <Tag color={v === 'openai' ? '#3b82f6' : '#a855f7'}>{v}</Tag>,
    },
    { title: 'Status', dataIndex: 'status_code', key: 'status_code', width: 80,
      render: (v: number) => <Tag color={v < 400 ? '#06d6a0' : v < 500 ? '#f59e0b' : '#ef4444'}>{v}</Tag>,
    },
    { title: 'Latency', dataIndex: 'latency_ms', key: 'latency_ms', width: 100,
      render: (v: number) => <span className="mono">{v}ms</span>,
    },
    { title: 'Input', dataIndex: 'input_tokens', key: 'input_tokens', width: 80,
      render: (v: number | null) => <span className="mono">{v ?? '-'}</span>,
    },
    { title: 'Output', dataIndex: 'output_tokens', key: 'output_tokens', width: 80,
      render: (v: number | null) => <span className="mono">{v ?? '-'}</span>,
    },
    {
      title: 'Actions', key: 'actions', width: 80,
      render: (_: unknown, record: AuditLog) => (
        <a onClick={() => setSelectedLog(record)}>View</a>
      ),
    },
  ];

  return (
    <div>
      <div className="page-header">
        <h1 className="page-title">Audit Logs</h1>
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

      <div className="console-table">
        <Table
          dataSource={data?.items}
          columns={columns}
          rowKey="id"
          loading={isLoading}
          size="small"
          scroll={{ x: 1000 }}
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

      <Drawer
        title="Log Detail"
        width={700}
        open={!!selectedLog}
        onClose={() => setSelectedLog(null)}
      >
        {selectedLog && (
          <div>
            <p><strong>Time:</strong> <span className="mono">{new Date(selectedLog.created_at).toLocaleString()}</span></p>
            <p><strong>Model:</strong> <span className="mono">{selectedLog.model_name}</span></p>
            <p><strong>Protocol:</strong> <Tag color={selectedLog.protocol === 'openai' ? '#3b82f6' : '#a855f7'}>{selectedLog.protocol}</Tag></p>
            <p><strong>Status:</strong> <Tag color={selectedLog.status_code < 400 ? '#06d6a0' : '#ef4444'}>{selectedLog.status_code}</Tag></p>
            <p><strong>Latency:</strong> <span className="mono">{selectedLog.latency_ms}ms</span></p>
            <p><strong>Tokens:</strong> <span className="mono">{selectedLog.input_tokens ?? 0} in / {selectedLog.output_tokens ?? 0} out</span></p>

            <Title level={5} style={{ marginTop: 16, fontFamily: 'var(--font-display)' }}>Request Body</Title>
            <JsonViewer data={selectedLog.request_body} />

            <Title level={5} style={{ marginTop: 16, fontFamily: 'var(--font-display)' }}>Response Body</Title>
            <JsonViewer data={selectedLog.response_body} />
          </div>
        )}
      </Drawer>
    </div>
  );
}
