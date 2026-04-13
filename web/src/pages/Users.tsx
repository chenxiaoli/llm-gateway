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
      <div className="mb-6">
        <h1 className="font-display text-2xl font-bold text-[#ededed]">Users</h1>
      </div>

      {isLoading ? (
        <div className="flex items-center justify-center py-12 text-[#555555]">Loading...</div>
      ) : (
        <>
          <div className="overflow-x-auto rounded-xl border border-[#1e1e1e]">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-[#1e1e1e]">
                  <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Username</th>
                  <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Role</th>
                  <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Status</th>
                  <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Created</th>
                  <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Actions</th>
                </tr>
              </thead>
              <tbody>
                {data?.items?.map((user) => (
                  <tr key={user.id} className="border-b border-[#1e1e1e]/50 hover:bg-white/[0.02] transition-colors">
                    <td className="px-4 py-2.5">{user.username}</td>
                    <td className="px-4 py-2.5">
                      <Select
                        value={user.role}
                        size="sm"
                        onChange={(value) => updateMutation.mutate({ id: user.id, input: { role: value as 'admin' | 'user' } })}
                        options={[
                          { value: 'admin', label: 'Admin' },
                          { value: 'user', label: 'User' },
                        ]}
                      />
                    </td>
                    <td className="px-4 py-2.5">
                      <button
                        className="cursor-pointer"
                        onClick={() => updateMutation.mutate({ id: user.id, input: { enabled: !user.enabled } })}
                      >
                        <Badge variant={user.enabled ? 'green' : 'red'}>{user.enabled ? 'Enabled' : 'Disabled'}</Badge>
                      </button>
                    </td>
                    <td className="px-4 py-2.5"><span className="mono">{new Date(user.created_at).toLocaleDateString()}</span></td>
                    <td className="px-4 py-2.5">
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
              <span className="text-[#555555]">Total {data?.total ?? 0}</span>
              <div className="flex gap-2">
                <Button variant="ghost" size="sm" disabled={page <= 1} onClick={() => setPage(page - 1)}>Previous</Button>
                <span className="px-2 text-[#888888]">{page} / {totalPages}</span>
                <Button variant="ghost" size="sm" disabled={page >= totalPages} onClick={() => setPage(page + 1)}>Next</Button>
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
}
