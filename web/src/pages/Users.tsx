import { useState } from 'react';
import { useUsers, useUpdateUser, useDeleteUser } from '../hooks/useUsers';
import { Button } from '../components/ui/Button';
import { Badge } from '../components/ui/Badge';
import { Select } from '../components/ui/Select';
import { ConfirmDialog } from '../components/ui/ConfirmDialog';

export default function Users() {
  const [page, setPage] = useState(1);
  const [pageSize] = useState(20);
  const { data, isLoading } = useUsers(page, pageSize);
  const updateMutation = useUpdateUser();
  const deleteMutation = useDeleteUser();

  const totalPages = Math.ceil((data?.total ?? 0) / pageSize);

  return (
    <div>
      <div className="mb-6"><h1 className="text-2xl font-bold">Users</h1></div>

      {isLoading ? (
        <div className="flex items-center justify-center py-12"><span className="loading loading-spinner loading-lg" /></div>
      ) : (
        <>
          <div className="overflow-x-auto bg-base-100 rounded-box shadow-sm">
            <table className="table table-sm">
              <thead>
                <tr className="border-b border-base-300">
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Username</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Role</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Status</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Created</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Actions</th>
                </tr>
              </thead>
              <tbody>
                {data?.items?.map((user) => (
                  <tr key={user.id} className="border-b border-base-200 hover">
                    <td>{user.username}</td>
                    <td>
                      <Select
                        value={user.role}
                        size="sm"
                        onChange={(value) => updateMutation.mutate({ id: user.id, input: { role: value as 'admin' | 'user' } })}
                        options={[{ value: 'admin', label: 'Admin' }, { value: 'user', label: 'User' }]}
                      />
                    </td>
                    <td>
                      <button className="cursor-pointer" onClick={() => updateMutation.mutate({ id: user.id, input: { enabled: !user.enabled } })}>
                        <Badge variant={user.enabled ? 'green' : 'red'}>{user.enabled ? 'Enabled' : 'Disabled'}</Badge>
                      </button>
                    </td>
                    <td className="mono">{new Date(user.created_at).toLocaleDateString()}</td>
                    <td>
                      <ConfirmDialog title="Delete this user?" onConfirm={() => deleteMutation.mutate(user.id)} okText="Delete">
                        <Button variant="danger" size="sm">Delete</Button>
                      </ConfirmDialog>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          {totalPages > 1 && (
            <div className="mt-4 flex items-center justify-between text-sm">
              <span className="text-base-content/40">Total {data?.total ?? 0}</span>
              <div className="join">
                <Button variant="ghost" size="sm" disabled={page <= 1} onClick={() => setPage(page - 1)}>Previous</Button>
                <span className="px-3 flex items-center text-base-content/60">{page} / {totalPages}</span>
                <Button variant="ghost" size="sm" disabled={page >= totalPages} onClick={() => setPage(page + 1)}>Next</Button>
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
}
