import { useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import {
  Card, Form, Input, Switch, Button, Space, Table, Modal,
  Popconfirm, Select, InputNumber, Tag,
} from 'antd';
import { ArrowLeftOutlined, PlusOutlined } from '@ant-design/icons';
import { useProvider, useUpdateProvider, useDeleteProvider, useChannels, useCreateChannel, useUpdateChannel, useDeleteChannel } from '../hooks/useProviders';
import { useModels, useCreateModel, useUpdateModel, useDeleteModel } from '../hooks/useModels';
import type { Model, CreateModelRequest, UpdateModelRequest, Channel } from '../types';

export default function ProviderDetail() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { data: provider, isLoading } = useProvider(id!);
  const updateMutation = useUpdateProvider();
  const deleteMutation = useDeleteProvider();
  const createModelMutation = useCreateModel(id!);
  const updateModelMutation = useUpdateModel(id!);
  const deleteModelMutation = useDeleteModel(id!);

  const [form] = Form.useForm();
  const [modelModalOpen, setModelModalOpen] = useState(false);
  const [editingModel, setEditingModel] = useState<Model | null>(null);
  const [modelForm] = Form.useForm();

  const createChannelMutation = useCreateChannel(id!);
  const updateChannelMutation = useUpdateChannel(id!);
  const deleteChannelMutation = useDeleteChannel(id!);
  const [channelModalOpen, setChannelModalOpen] = useState(false);
  const [editingChannel, setEditingChannel] = useState<Channel | null>(null);
  const [channelForm] = Form.useForm();

  const { data: channels } = useChannels(id!);
  const { data: models } = useModels(id!);

  if (isLoading) return <div>Loading...</div>;
  if (!provider) return <div>Provider not found</div>;

  const handleUpdateProvider = async (values: any) => {
    await updateMutation.mutateAsync({
      id: provider.id,
      input: {
        name: values.name,
        openai_base_url: values.openai_base_url || null,
        anthropic_base_url: values.anthropic_base_url || null,
        enabled: values.enabled,
      },
    });
  };

  const handleDeleteProvider = async () => {
    await deleteMutation.mutateAsync(provider.id);
    navigate('/console/providers');
  };

  const openAddModel = () => {
    setEditingModel(null);
    modelForm.resetFields();
    setModelModalOpen(true);
  };

  const openEditModel = (model: Model) => {
    setEditingModel(model);
    modelForm.setFieldsValue(model);
    setModelModalOpen(true);
  };

  const handleSaveModel = async (values: any) => {
    if (editingModel) {
      const updateInput: UpdateModelRequest = {
        billing_type: values.billing_type,
        input_price: values.input_price,
        output_price: values.output_price,
        request_price: values.request_price,
        enabled: values.enabled,
      };
      await updateModelMutation.mutateAsync({ modelName: editingModel.name, input: updateInput });
    } else {
      const input: CreateModelRequest = {
        name: values.name,
        billing_type: values.billing_type,
        input_price: values.input_price ?? 0,
        output_price: values.output_price ?? 0,
        request_price: values.request_price ?? 0,
      };
      await createModelMutation.mutateAsync(input);
    }
    setModelModalOpen(false);
  };

  const handleDeleteModel = async (modelName: string) => {
    await deleteModelMutation.mutateAsync(modelName);
  };

  const openAddChannel = () => {
    setEditingChannel(null);
    channelForm.resetFields();
    setChannelModalOpen(true);
  };

  const openEditChannel = (channel: Channel) => {
    setEditingChannel(channel);
    channelForm.setFieldsValue({
      name: channel.name,
      api_key: channel.api_key,
      base_url: channel.base_url,
      priority: channel.priority,
      enabled: channel.enabled,
    });
    setChannelModalOpen(true);
  };

  const handleSaveChannel = async (values: any) => {
    if (editingChannel) {
      await updateChannelMutation.mutateAsync({
        id: editingChannel.id,
        input: {
          name: values.name,
          api_key: values.api_key,
          base_url: values.base_url || null,
          priority: values.priority,
          enabled: values.enabled,
        },
      });
    } else {
      await createChannelMutation.mutateAsync({
        name: values.name,
        api_key: values.api_key,
        base_url: values.base_url || null,
        priority: values.priority,
      });
    }
    setChannelModalOpen(false);
  };

  const handleDeleteChannel = async (channelId: string) => {
    await deleteChannelMutation.mutateAsync(channelId);
  };

  const modelColumns = [
    { title: 'Name', dataIndex: 'name', key: 'name',
      render: (v: string) => <span className="mono">{v}</span>,
    },
    {
      title: 'Billing', dataIndex: 'billing_type', key: 'billing_type',
      render: (v: string) => <Tag color={v === 'token' ? '#3b82f6' : '#06d6a0'}>{v}</Tag>,
    },
    {
      title: 'Input ($/1M)', dataIndex: 'input_price', key: 'input_price',
      render: (v: number) => <span className="mono">{v.toFixed(2)}</span>,
    },
    {
      title: 'Output ($/1M)', dataIndex: 'output_price', key: 'output_price',
      render: (v: number) => <span className="mono">{v.toFixed(2)}</span>,
    },
    {
      title: 'Status', dataIndex: 'enabled', key: 'enabled',
      render: (enabled: boolean) => <Tag color={enabled ? '#06d6a0' : '#ef4444'}>{enabled ? 'Active' : 'Disabled'}</Tag>,
    },
    {
      title: 'Actions', key: 'actions',
      render: (_: unknown, record: Model) => (
        <Space>
          <a onClick={() => openEditModel(record)}>Edit</a>
          <Popconfirm title={`Delete model "${record.name}"?`} onConfirm={() => handleDeleteModel(record.name)}>
            <a style={{ color: '#ef4444' }}>Delete</a>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  const channelColumns = [
    { title: 'Name', dataIndex: 'name', key: 'name' },
    {
      title: 'Base URL', dataIndex: 'base_url', key: 'base_url',
      render: (v: string | null) => v ? <span className="mono">{v}</span> : <span style={{ color: 'var(--text-muted)' }}>Default</span>,
    },
    { title: 'Priority', dataIndex: 'priority', key: 'priority',
      render: (v: number) => <span className="mono">{v}</span>,
    },
    {
      title: 'Status', dataIndex: 'enabled', key: 'enabled',
      render: (enabled: boolean) => <Tag color={enabled ? '#06d6a0' : '#ef4444'}>{enabled ? 'Active' : 'Disabled'}</Tag>,
    },
    {
      title: 'Actions', key: 'actions',
      render: (_: unknown, record: Channel) => (
        <Space>
          <a onClick={() => openEditChannel(record)}>Edit</a>
          <Popconfirm title={`Delete channel "${record.name}"?`} onConfirm={() => handleDeleteChannel(record.id)}>
            <a style={{ color: '#ef4444' }}>Delete</a>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  return (
    <div>
      <Button icon={<ArrowLeftOutlined />} onClick={() => navigate('/console/providers')} style={{ marginBottom: 16 }}>
        Back to Providers
      </Button>

      <div className="page-header">
        <h1 className="page-title">Provider: {provider.name}</h1>
      </div>

      <Card style={{ marginBottom: 24, maxWidth: 500 }}>
        <Form
          form={form}
          layout="vertical"
          initialValues={{
            name: provider.name,
            openai_base_url: provider.openai_base_url,
            anthropic_base_url: provider.anthropic_base_url,
            enabled: provider.enabled,
          }}
          onFinish={handleUpdateProvider}
        >
          <Form.Item name="name" label="Name" rules={[{ required: true }]}>
            <Input />
          </Form.Item>
          <Form.Item name="openai_base_url" label="OpenAI Base URL">
            <Input />
          </Form.Item>
          <Form.Item name="anthropic_base_url" label="Anthropic Base URL">
            <Input />
          </Form.Item>
          <Form.Item name="enabled" label="Enabled" valuePropName="checked">
            <Switch />
          </Form.Item>
          <Form.Item>
            <Space>
              <Button type="primary" htmlType="submit" loading={updateMutation.isPending}>Save</Button>
              <Popconfirm title="Delete this provider and all its models?" onConfirm={handleDeleteProvider}>
                <Button danger>Delete Provider</Button>
              </Popconfirm>
            </Space>
          </Form.Item>
        </Form>
      </Card>

      <Card
        title={<h3 className="page-title" style={{ fontSize: 16, margin: 0 }}>Models</h3>}
        extra={<Button type="primary" icon={<PlusOutlined />} onClick={openAddModel}>Add Model</Button>}
        style={{ marginBottom: 24 }}
      >
        <div className="console-table">
          <Table dataSource={models ?? []} columns={modelColumns} rowKey="name" pagination={false} />
        </div>
      </Card>

      <Card
        title={<h3 className="page-title" style={{ fontSize: 16, margin: 0 }}>Channels</h3>}
        extra={<Button type="primary" icon={<PlusOutlined />} onClick={openAddChannel}>Add Channel</Button>}
      >
        <div className="console-table">
          <Table dataSource={channels} columns={channelColumns} rowKey="id" pagination={false} />
        </div>
      </Card>

      <Modal
        title={editingModel ? `Edit Model: ${editingModel.name}` : 'Add Model'}
        open={modelModalOpen}
        onCancel={() => setModelModalOpen(false)}
        footer={null}
      >
        <Form form={modelForm} layout="vertical" onFinish={handleSaveModel}>
          <Form.Item name="name" label="Model Name" rules={[{ required: true }]} hidden={!!editingModel}>
            <Input placeholder="e.g., gpt-4o" />
          </Form.Item>
          <Form.Item name="billing_type" label="Billing Type" rules={[{ required: true }]}>
            <Select options={[
              { value: 'token', label: 'Token-based' },
              { value: 'request', label: 'Request-based' },
            ]} />
          </Form.Item>
          <Space>
            <Form.Item name="input_price" label="Input Price ($/1M tokens)">
              <InputNumber min={0} step={0.01} style={{ width: 150 }} />
            </Form.Item>
            <Form.Item name="output_price" label="Output Price ($/1M tokens)">
              <InputNumber min={0} step={0.01} style={{ width: 150 }} />
            </Form.Item>
            <Form.Item name="request_price" label="Request Price ($/req)">
              <InputNumber min={0} step={0.01} style={{ width: 150 }} />
            </Form.Item>
          </Space>
          {editingModel && (
            <Form.Item name="enabled" label="Enabled" valuePropName="checked">
              <Switch />
            </Form.Item>
          )}
          <Form.Item>
            <Button type="primary" htmlType="submit">
              {editingModel ? 'Update' : 'Create'}
            </Button>
          </Form.Item>
        </Form>
      </Modal>

      <Modal
        title={editingChannel ? `Edit Channel: ${editingChannel.name}` : 'Add Channel'}
        open={channelModalOpen}
        onCancel={() => setChannelModalOpen(false)}
        footer={null}
      >
        <Form form={channelForm} layout="vertical" onFinish={handleSaveChannel}>
          <Form.Item name="name" label="Name" rules={[{ required: true }]}>
            <Input placeholder="e.g., primary" />
          </Form.Item>
          <Form.Item name="api_key" label="API Key" rules={[{ required: true }]}>
            <Input.Password placeholder="Upstream API key" />
          </Form.Item>
          <Form.Item name="base_url" label="Base URL">
            <Input placeholder="Leave empty to use provider default" />
          </Form.Item>
          <Form.Item name="priority" label="Priority" initialValue={0}>
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
          {editingChannel && (
            <Form.Item name="enabled" label="Enabled" valuePropName="checked">
              <Switch />
            </Form.Item>
          )}
          <Form.Item>
            <Button type="primary" htmlType="submit" loading={createChannelMutation.isPending || updateChannelMutation.isPending}>
              {editingChannel ? 'Update' : 'Create'}
            </Button>
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
}
