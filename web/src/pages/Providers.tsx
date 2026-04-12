import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Table, Button, Modal, Form, Input, Space, Tag, Typography } from 'antd';
import { PlusOutlined } from '@ant-design/icons';
import { useProviders, useCreateProvider } from '../hooks/useProviders';

const { Title } = Typography;

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
        <a onClick={() => navigate(`/admin/providers/${record.id}`)}>{name}</a>
      ),
    },
    {
      title: 'Protocols',
      key: 'protocols',
      render: (_: unknown, record: { openai_base_url: string | null; anthropic_base_url: string | null }) => (
        <Space>
          {record.openai_base_url && <Tag color="blue">OpenAI</Tag>}
          {record.anthropic_base_url && <Tag color="purple">Anthropic</Tag>}
        </Space>
      ),
    },
    {
      title: 'Status',
      dataIndex: 'enabled',
      key: 'enabled',
      render: (enabled: boolean) => (
        <Tag color={enabled ? 'green' : 'red'}>{enabled ? 'Active' : 'Disabled'}</Tag>
      ),
    },
    {
      title: 'Created',
      dataIndex: 'created_at',
      key: 'created_at',
      render: (v: string) => new Date(v).toLocaleDateString(),
    },
  ];

  return (
    <>
      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 16 }}>
        <Title level={4} style={{ margin: 0 }}>Providers</Title>
        <Button type="primary" icon={<PlusOutlined />} onClick={() => setCreateOpen(true)}>
          Add Provider
        </Button>
      </div>

      <Table dataSource={providers} columns={columns} rowKey="id" loading={isLoading} />

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
