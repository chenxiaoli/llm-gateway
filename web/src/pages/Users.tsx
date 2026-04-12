import { Table, Tag, Button, Popconfirm, Select, Typography } from 'antd';
import { useUsers, useUpdateUser, useDeleteUser } from '../hooks/useUsers';
import type { UserResponse } from '../types';

const { Title } = Typography;

export default function Users() {
  const { data: users, isLoading } = useUsers();
  const updateMutation = useUpdateUser();
  const deleteMutation = useDeleteUser();

  const columns = [
    {
      title: 'Username',
      dataIndex: 'username',
      key: 'username',
    },
    {
      title: 'Role',
      dataIndex: 'role',
      key: 'role',
      render: (role: string, record: UserResponse) => (
        <Select
          value={role}
          size="small"
          style={{ width: 100 }}
          onChange={(value) => updateMutation.mutate({ id: record.id, input: { role: value as 'admin' | 'user' } })}
          options={[
            { value: 'admin', label: 'Admin' },
            { value: 'user', label: 'User' },
          ]}
        />
      ),
    },
    {
      title: 'Status',
      dataIndex: 'enabled',
      key: 'enabled',
      render: (enabled: boolean, record: UserResponse) => (
        <Tag
          color={enabled ? 'green' : 'red'}
          style={{ cursor: 'pointer' }}
          onClick={() => updateMutation.mutate({ id: record.id, input: { enabled: !enabled } })}
        >
          {enabled ? 'Enabled' : 'Disabled'}
        </Tag>
      ),
    },
    {
      title: 'Created',
      dataIndex: 'created_at',
      key: 'created_at',
      render: (v: string) => new Date(v).toLocaleDateString(),
    },
    {
      title: 'Actions',
      key: 'actions',
      render: (_: unknown, record: UserResponse) => (
        <Popconfirm
          title="Delete this user?"
          onConfirm={() => deleteMutation.mutate(record.id)}
          okText="Delete"
          cancelText="Cancel"
        >
          <Button danger size="small">Delete</Button>
        </Popconfirm>
      ),
    },
  ];

  return (
    <div>
      <Title level={4}>Users</Title>
      <Table dataSource={users} columns={columns} rowKey="id" loading={isLoading} />
    </div>
  );
}
