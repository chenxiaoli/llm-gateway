import { useParams, useNavigate } from 'react-router-dom';
import { Card, Form, Input, InputNumber, Switch, Button, Space, Typography, Popconfirm } from 'antd';
import { ArrowLeftOutlined } from '@ant-design/icons';
import { useKey, useUpdateKey, useDeleteKey } from '../hooks/useKeys';

const { Title } = Typography;

export default function KeyDetail() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { data: key, isLoading } = useKey(id!);
  const updateMutation = useUpdateKey();
  const deleteMutation = useDeleteKey();
  const [form] = Form.useForm();

  if (isLoading) return <div>Loading...</div>;
  if (!key) return <div>Key not found</div>;

  const handleUpdate = async (values: any) => {
    await updateMutation.mutateAsync({
      id: key.id,
      input: {
        name: values.name,
        rate_limit: values.rate_limit ?? null,
        budget_monthly: values.budget_monthly ?? null,
        enabled: values.enabled,
      },
    });
  };

  const handleDelete = async () => {
    await deleteMutation.mutateAsync(key.id);
    navigate('/admin/keys');
  };

  return (
    <div>
      <Button icon={<ArrowLeftOutlined />} onClick={() => navigate('/admin/keys')} style={{ marginBottom: 16 }}>
        Back to Keys
      </Button>

      <Card title={<Title level={4} style={{ margin: 0 }}>Edit Key: {key.name}</Title>}>
        <Form
          form={form}
          layout="vertical"
          initialValues={{
            name: key.name,
            rate_limit: key.rate_limit,
            budget_monthly: key.budget_monthly,
            enabled: key.enabled,
          }}
          onFinish={handleUpdate}
          style={{ maxWidth: 500 }}
        >
          <Form.Item name="name" label="Name" rules={[{ required: true }]}>
            <Input />
          </Form.Item>
          <Form.Item name="enabled" label="Enabled" valuePropName="checked">
            <Switch />
          </Form.Item>
          <Form.Item name="rate_limit" label="Rate Limit (RPM, null = unlimited)">
            <InputNumber min={1} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="budget_monthly" label="Monthly Budget ($, null = unlimited)">
            <InputNumber min={0} step={0.01} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item>
            <Space>
              <Button type="primary" htmlType="submit" loading={updateMutation.isPending}>Save</Button>
              <Popconfirm title="Delete this key?" onConfirm={handleDelete}>
                <Button danger>Delete Key</Button>
              </Popconfirm>
            </Space>
          </Form.Item>
        </Form>
      </Card>
    </div>
  );
}
