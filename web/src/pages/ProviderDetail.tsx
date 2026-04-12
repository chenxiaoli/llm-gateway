import { useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import {
  Card, Form, Input, Switch, Button, Space, Table, Modal,
  Popconfirm, Typography, Select, InputNumber, Tag,
} from 'antd';
import { ArrowLeftOutlined, PlusOutlined } from '@ant-design/icons';
import { useProvider, useUpdateProvider, useDeleteProvider } from '../hooks/useProviders';
import { useCreateModel, useUpdateModel, useDeleteModel } from '../hooks/useModels';
import type { Model, CreateModelRequest, UpdateModelRequest } from '../types';

const { Title } = Typography;

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

  if (isLoading) return <div>Loading...</div>;
  if (!provider) return <div>Provider not found</div>;

  const handleUpdateProvider = async (values: any) => {
    await updateMutation.mutateAsync({
      id: provider.id,
      input: {
        name: values.name,
        api_key: values.api_key,
        openai_base_url: values.openai_base_url || null,
        anthropic_base_url: values.anthropic_base_url || null,
        enabled: values.enabled,
      },
    });
  };

  const handleDeleteProvider = async () => {
    await deleteMutation.mutateAsync(provider.id);
    navigate('/admin/providers');
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

  const modelColumns = [
    { title: 'Name', dataIndex: 'name', key: 'name' },
    {
      title: 'Billing', dataIndex: 'billing_type', key: 'billing_type',
      render: (v: string) => <Tag color={v === 'token' ? 'blue' : 'green'}>{v}</Tag>,
    },
    {
      title: 'Input Price ($/1M)', dataIndex: 'input_price', key: 'input_price',
      render: (v: number) => v.toFixed(2),
    },
    {
      title: 'Output Price ($/1M)', dataIndex: 'output_price', key: 'output_price',
      render: (v: number) => v.toFixed(2),
    },
    {
      title: 'Status', dataIndex: 'enabled', key: 'enabled',
      render: (enabled: boolean) => <Tag color={enabled ? 'green' : 'red'}>{enabled ? 'Active' : 'Disabled'}</Tag>,
    },
    {
      title: 'Actions', key: 'actions',
      render: (_: unknown, record: Model) => (
        <Space>
          <a onClick={() => openEditModel(record)}>Edit</a>
          <Popconfirm title={`Delete model "${record.name}"?`} onConfirm={() => handleDeleteModel(record.name)}>
            <a style={{ color: '#ff4d4f' }}>Delete</a>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  return (
    <div>
      <Button icon={<ArrowLeftOutlined />} onClick={() => navigate('/admin/providers')} style={{ marginBottom: 16 }}>
        Back to Providers
      </Button>

      <Card title={<Title level={4} style={{ margin: 0 }}>Provider: {provider.name}</Title>} style={{ marginBottom: 16 }}>
        <Form
          form={form}
          layout="vertical"
          initialValues={{
            name: provider.name,
            api_key: provider.api_key,
            openai_base_url: provider.openai_base_url,
            anthropic_base_url: provider.anthropic_base_url,
            enabled: provider.enabled,
          }}
          onFinish={handleUpdateProvider}
          style={{ maxWidth: 500 }}
        >
          <Form.Item name="name" label="Name" rules={[{ required: true }]}>
            <Input />
          </Form.Item>
          <Form.Item name="api_key" label="API Key" rules={[{ required: true }]}>
            <Input.Password />
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
        title={<Title level={4} style={{ margin: 0 }}>Models</Title>}
        extra={<Button type="primary" icon={<PlusOutlined />} onClick={openAddModel}>Add Model</Button>}
      >
        <Table dataSource={[]} columns={modelColumns} rowKey="name" pagination={false} />
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
    </div>
  );
}
