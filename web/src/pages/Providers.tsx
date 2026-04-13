import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Table, Button, Modal, Form, Input, Space, Tag } from 'antd';
import { PlusOutlined } from '@ant-design/icons';
import { useProviders, useCreateProvider } from '../hooks/useProviders';

export default function Providers() {
  const { data: providers, isLoading } = useProviders();
  const createMutation = useCreateProvider();
  const navigate = useNavigate();
  const [createOpen, setCreateOpen] = useState(false);
  const [form] = Form.useForm();

  const handleCreate = async (values: { name: string; openai_base_url?: string; anthropic_base_url?: string }) => {
    await createMutation.mutateAsync({
      name: values.name,
      openai_base_url: values.openai_base_url || null,
      anthropic_base_url: values.anthropic_base_url || null,
    });
    form.resetFields();
    setCreateOpen(false);
  };

  const columns = [
    {
      title: 'Name',
      dataIndex: 'name',
      key: 'name',
      render: (name: string, record: { id: string }) => (
        <a onClick={() => navigate(`/console/providers/${record.id}`)}>{name}</a>
      ),
    },
    {
      title: 'Protocols',
      key: 'protocols',
      render: (_: unknown, record: { openai_base_url: string | null; anthropic_base_url: string | null }) => (
        <Space>
          {record.openai_base_url && <Tag color="#3b82f6">OpenAI</Tag>}
          {record.anthropic_base_url && <Tag color="#a855f7">Anthropic</Tag>}
        </Space>
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
      title: 'Created',
      dataIndex: 'created_at',
      key: 'created_at',
      render: (v: string) => <span className="mono">{new Date(v).toLocaleDateString()}</span>,
    },
  ];

  return (
    <>
      <div className="page-header" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <h1 className="page-title">Providers</h1>
        <Button type="primary" icon={<PlusOutlined />} onClick={() => setCreateOpen(true)}>
          Add Provider
        </Button>
      </div>

      <div className="console-table">
        <Table dataSource={providers} columns={columns} rowKey="id" loading={isLoading} />
      </div>

      <Modal
        title="Add Provider"
        open={createOpen}
        onCancel={() => setCreateOpen(false)}
        footer={null}
      >
        <Form form={form} layout="vertical" onFinish={handleCreate}>
          <Form.Item name="name" label="Name" rules={[{ required: true }]}>
            <Input placeholder="e.g., OpenAI" />
          </Form.Item>
          <Form.Item name="openai_base_url" label="OpenAI Base URL">
            <Input placeholder="https://api.openai.com/v1" />
          </Form.Item>
          <Form.Item name="anthropic_base_url" label="Anthropic Base URL">
            <Input placeholder="https://api.anthropic.com" />
          </Form.Item>
          <Form.Item>
            <Button type="primary" htmlType="submit" loading={createMutation.isPending}>
              Create
            </Button>
          </Form.Item>
        </Form>
      </Modal>
    </>
  );
}
