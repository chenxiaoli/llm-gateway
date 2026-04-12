import { useState } from 'react';
import { Typography, Switch, Card, Form, Input, Button, Alert } from 'antd';
import { useSettings, useUpdateSettings } from '../hooks/useSettings';
import { changePassword } from '../api/auth';

const { Title } = Typography;

export default function Settings() {
  const { data: settings, isLoading } = useSettings();
  const updateMutation = useUpdateSettings();
  const [passwordLoading, setPasswordLoading] = useState(false);
  const [passwordStatus, setPasswordStatus] = useState<{ type: 'success' | 'error'; message: string } | null>(null);
  const [passwordForm] = Form.useForm();

  const handleChangePassword = async (values: { current_password: string; new_password: string }) => {
    setPasswordLoading(true);
    setPasswordStatus(null);
    try {
      await changePassword(values);
      setPasswordStatus({ type: 'success', message: 'Password changed successfully' });
      passwordForm.resetFields();
    } catch {
      setPasswordStatus({ type: 'error', message: 'Failed to change password' });
    } finally {
      setPasswordLoading(false);
    }
  };

  return (
    <div>
      <Title level={4}>Settings</Title>
      <Card loading={isLoading}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', maxWidth: 400 }}>
          <span>Allow Registration</span>
          <Switch
            checked={settings?.allow_registration ?? false}
            onChange={(checked) => updateMutation.mutate({ allow_registration: checked })}
          />
        </div>
      </Card>

      <Card title="Change Password" style={{ marginTop: 16, maxWidth: 400 }}>
        {passwordStatus && (
          <Alert
            type={passwordStatus.type}
            message={passwordStatus.message}
            showIcon
            closable
            style={{ marginBottom: 16 }}
            onClose={() => setPasswordStatus(null)}
          />
        )}
        <Form form={passwordForm} onFinish={handleChangePassword} layout="vertical">
          <Form.Item
            name="current_password"
            label="Current Password"
            rules={[{ required: true, message: 'Please enter your current password' }]}
          >
            <Input.Password />
          </Form.Item>
          <Form.Item
            name="new_password"
            label="New Password"
            rules={[{ required: true, message: 'Please enter a new password' }]}
          >
            <Input.Password />
          </Form.Item>
          <Form.Item
            name="confirm_password"
            label="Confirm New Password"
            dependencies={['new_password']}
            rules={[
              { required: true, message: 'Please confirm your new password' },
              ({ getFieldValue }) => ({
                validator(_, value) {
                  if (!value || getFieldValue('new_password') === value) {
                    return Promise.resolve();
                  }
                  return Promise.reject(new Error('Passwords do not match'));
                },
              }),
            ]}
          >
            <Input.Password />
          </Form.Item>
          <Form.Item>
            <Button type="primary" htmlType="submit" loading={passwordLoading}>
              Change Password
            </Button>
          </Form.Item>
        </Form>
      </Card>
    </div>
  );
}
