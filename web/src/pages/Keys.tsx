import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Table, Button, Modal, Form, Input, InputNumber, Space, Tag, Typography, message } from 'antd';
import { PlusOutlined } from '@ant-design/icons';
import { useKeys, useCreateKey } from '../hooks/useKeys';
import type { CreateKeyResponse } from '../types';

const { Text, Paragraph } = Typography;

export default function Keys() {
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(20);
  const { data, isLoading } = useKeys(page, pageSize);
  const createKeyMutation = useCreateKey();
  const navigate = useNavigate();
  const [createOpen, setCreateOpen] = useState(false);
  const [createdKey, setCreatedKey] = useState<string | null>(null);
  const [form] = Form.useForm();

  const handleCreate = async (values: { name: string; rate_limit?: number; budget_monthly?: number }) => {
    const result: CreateKeyResponse = await createKeyMutation.mutateAsync({
      name: values.name,
      rate_limit: values.rate_limit ?? null,
      budget_monthly: values.budget_monthly ?? null,
    });
    setCreatedKey(result.key);
    form.resetFields();
  };

  const copyKey = () => {
    if (createdKey) {
      navigator.clipboard.writeText(createdKey);
      message.success('Key copied to clipboard');
    }
  };

  const columns = [
    {
      title: 'Name',
      dataIndex: 'name',
      key: 'name',
      render: (name: string, record: { id: string }) => (
        <a onClick={() => navigate(`/console/keys/${record.id}`)}>{name}</a>
      ),
    },
    {
      title: 'Status',
      dataIndex: 'enabled',
      key: 'enabled',
      render: (enabled: boolean) => (
        <Tag color={enabled ? '#06d6a0' : '#ef4444'}>{enabled ? 'Active' : 'Disabled'}</Tag>
      ),
    },
    {
      title: 'Rate Limit (RPM)',
      dataIndex: 'rate_limit',
      key: 'rate_limit',
      render: (v: number | null) => <span className="mono">{v ?? 'Unlimited'}</span>,
    },
    {
      title: 'Monthly Budget',
      dataIndex: 'budget_monthly',
      key: 'budget_monthly',
      render: (v: number | null) => <span className="mono">{v != null ? `$${v.toFixed(2)}` : 'Unlimited'}</span>,
    },
    {
      title: 'Created',
      dataIndex: 'created_at',
      key: 'created_at',
      render: (v: string) => <span className="mono">{new Date(v).toLocaleDateString()}</span>,
    },
  ];

  return (
    <>
      <div className="page-header" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <h1 className="page-title">API Keys</h1>
        <Button type="primary" icon={<PlusOutlined />} onClick={() => setCreateOpen(true)}>
          Create Key
        </Button>
      </div>

      <div className="console-table">
        <Table
          dataSource={data?.items}
          columns={columns}
          rowKey="id"
          loading={isLoading}
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

      <Modal
        title="Create API Key"
        open={createOpen}
        onCancel={() => { setCreateOpen(false); setCreatedKey(null); }}
        footer={createdKey ? [
          <Button key="copy" type="primary" onClick={copyKey}>Copy Key</Button>,
          <Button key="done" onClick={() => { setCreateOpen(false); setCreatedKey(null); }}>Done</Button>,
        ] : undefined}
      >
        {createdKey ? (
          <div>
            <Text type="secondary">Save this key now. It won't be shown again.</Text>
            <Paragraph copyable style={{ marginTop: 8, fontFamily: 'var(--font-mono)', fontSize: 14 }}>
              {createdKey}
            </Paragraph>
          </div>
        ) : (
          <Form form={form} layout="vertical" onFinish={handleCreate}>
            <Form.Item name="name" label="Name" rules={[{ required: true, message: 'Enter a name' }]}>
              <Input placeholder="e.g., production-app" />
            </Form.Item>
            <Space>
              <Form.Item name="rate_limit" label="Rate Limit (RPM)">
                <InputNumber min={1} placeholder="Unlimited" style={{ width: 150 }} />
              </Form.Item>
              <Form.Item name="budget_monthly" label="Monthly Budget ($)">
                <InputNumber min={0} step={0.01} placeholder="Unlimited" style={{ width: 150 }} />
              </Form.Item>
            </Space>
            <Form.Item>
              <Button type="primary" htmlType="submit" loading={createKeyMutation.isPending}>
                Create
              </Button>
            </Form.Item>
          </Form>
        )}
      </Modal>
    </>
  );
}
