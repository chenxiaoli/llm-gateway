import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Plus } from 'lucide-react';
import { useKeys, useCreateKey } from '../hooks/useKeys';
import { Button } from '../components/ui/Button';
import { Modal } from '../components/ui/Modal';
import { Badge } from '../components/ui/Badge';
import { toast } from 'sonner';
import type { CreateKeyResponse } from '../types';

export default function Keys() {
  const [page, setPage] = useState(1);
  const [pageSize] = useState(20);
  const { data, isLoading } = useKeys(page, pageSize);
  const createKeyMutation = useCreateKey();
  const navigate = useNavigate();
  const [createOpen, setCreateOpen] = useState(false);
  const [createdKey, setCreatedKey] = useState<string | null>(null);
  const [name, setName] = useState('');
  const [rateLimit, setRateLimit] = useState('');
  const [budget, setBudget] = useState('');

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    const result: CreateKeyResponse = await createKeyMutation.mutateAsync({
      name,
      rate_limit: rateLimit ? Number(rateLimit) : null,
      budget_monthly: budget ? Number(budget) : null,
    });
    setCreatedKey(result.key);
    setName('');
    setRateLimit('');
    setBudget('');
  };

  const copyKey = () => {
    if (createdKey) {
      navigator.clipboard.writeText(createdKey);
      toast.success('Key copied to clipboard');
    }
  };

  const totalPages = Math.ceil((data?.total ?? 0) / pageSize);

  return (
    <div>
      <div className="mb-8 flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">API Keys</h1>
          <p className="text-sm text-base-content/40 mt-1">Manage access keys for API authentication</p>
        </div>
        <Button icon={<Plus className="h-4 w-4" />} onClick={() => setCreateOpen(true)}>
          Create Key
        </Button>
      </div>

      {isLoading ? (
        <div className="flex items-center justify-center py-12"><span className="loading loading-spinner loading-lg" /></div>
      ) : (
        <div className="rounded-xl border border-base-300/50 bg-base-100/60 overflow-hidden">
          <div className="overflow-x-auto">
            <table className="table table-sm">
              <thead>
                <tr className="border-b border-base-300/50">
                  <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/35">Name</th>
                  <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/35">Status</th>
                  <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/35">Rate Limit (RPM)</th>
                  <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/35">Monthly Budget</th>
                  <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/35">Created</th>
                </tr>
              </thead>
              <tbody>
                {data?.items?.map((key) => (
                  <tr key={key.id} className="border-b border-base-200/50 hover:bg-base-200/30 transition-colors">
                    <td>
                      <button onClick={() => navigate(`/console/keys/${key.id}`)} className="link link-primary text-sm font-medium">
                        {key.name}
                      </button>
                    </td>
                    <td><Badge variant={key.enabled ? 'green' : 'red'}>{key.enabled ? 'Active' : 'Disabled'}</Badge></td>
                    <td className="mono text-[13px] text-base-content/60">{key.rate_limit ?? 'Unlimited'}</td>
                    <td className="mono text-[13px] text-base-content/60">{key.budget_monthly != null ? `$${key.budget_monthly.toFixed(2)}` : 'Unlimited'}</td>
                    <td className="mono text-[13px] text-base-content/50">{new Date(key.created_at).toLocaleDateString()}</td>
                  </tr>
                ))}
                {(!data?.items?.length) && (
                  <tr>
                    <td colSpan={5} className="text-center py-16 text-base-content/25 text-sm">
                      No API keys yet
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {totalPages > 1 && (
        <div className="mt-4 flex items-center justify-between text-sm">
          <span className="text-base-content/30">Total {data?.total ?? 0}</span>
          <div className="join">
            <Button variant="ghost" size="sm" disabled={page <= 1} onClick={() => setPage(page - 1)}>Previous</Button>
            <span className="px-3 flex items-center text-base-content/50">{page} / {totalPages}</span>
            <Button variant="ghost" size="sm" disabled={page >= totalPages} onClick={() => setPage(page + 1)}>Next</Button>
          </div>
        </div>
      )}

      <Modal
        open={createOpen}
        onClose={() => { setCreateOpen(false); setCreatedKey(null); }}
        title="Create API Key"
        footer={createdKey ? (
          <div className="flex gap-2">
            <Button onClick={copyKey}>Copy Key</Button>
            <Button variant="secondary" onClick={() => { setCreateOpen(false); setCreatedKey(null); }}>Done</Button>
          </div>
        ) : undefined}
      >
        {createdKey ? (
          <div>
            <p className="text-sm text-base-content/40">Save this key now. It won't be shown again.</p>
            <div className="mt-3 rounded-lg border border-base-300/50 bg-base-200/50 p-3 font-mono text-sm break-all">{createdKey}</div>
          </div>
        ) : (
          <form onSubmit={handleCreate} className="space-y-4">
            <div className="form-control">
              <label className="label"><span className="label-text font-medium">Name</span></label>
              <input
                type="text"
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="e.g., production-app"
                required
                className="input input-bordered w-full"
              />
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div className="form-control">
                <label className="label"><span className="label-text font-medium">Rate Limit (RPM)</span></label>
                <input
                  type="number"
                  value={rateLimit}
                  onChange={(e) => setRateLimit(e.target.value)}
                  placeholder="Unlimited"
                  min={1}
                  className="input input-bordered w-full"
                />
              </div>
              <div className="form-control">
                <label className="label"><span className="label-text font-medium">Monthly Budget ($)</span></label>
                <input
                  type="number"
                  value={budget}
                  onChange={(e) => setBudget(e.target.value)}
                  placeholder="Unlimited"
                  min={0}
                  step={0.01}
                  className="input input-bordered w-full"
                />
              </div>
            </div>
            <Button variant="primary" loading={createKeyMutation.isPending}>Create</Button>
          </form>
        )}
      </Modal>
    </div>
  );
}