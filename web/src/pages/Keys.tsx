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
      <div className="mb-6 flex items-center justify-between">
        <h1 className="font-display text-2xl font-bold text-[#ededed]">API Keys</h1>
        <Button icon={<Plus className="h-4 w-4" />} onClick={() => setCreateOpen(true)}>
          Create Key
        </Button>
      </div>

      {isLoading ? (
        <div className="flex items-center justify-center py-12 text-[#555555]">Loading...</div>
      ) : (
        <>
          <div className="overflow-x-auto rounded-xl border border-[#1e1e1e]">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-[#1e1e1e]">
                  <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Name</th>
                  <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Status</th>
                  <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Rate Limit (RPM)</th>
                  <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Monthly Budget</th>
                  <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Created</th>
                </tr>
              </thead>
              <tbody>
                {data?.items?.map((key) => (
                  <tr key={key.id} className="border-b border-[#1e1e1e]/50 hover:bg-white/[0.02] transition-colors">
                    <td className="px-4 py-2.5">
                      <button onClick={() => navigate(`/console/keys/${key.id}`)} className="text-accent hover:text-accent-hover transition-colors">
                        {key.name}
                      </button>
                    </td>
                    <td className="px-4 py-2.5"><Badge variant={key.enabled ? 'green' : 'red'}>{key.enabled ? 'Active' : 'Disabled'}</Badge></td>
                    <td className="px-4 py-2.5"><span className="mono">{key.rate_limit ?? 'Unlimited'}</span></td>
                    <td className="px-4 py-2.5"><span className="mono">{key.budget_monthly != null ? `$${key.budget_monthly.toFixed(2)}` : 'Unlimited'}</span></td>
                    <td className="px-4 py-2.5"><span className="mono">{new Date(key.created_at).toLocaleDateString()}</span></td>
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
            <p className="text-sm text-[#888888]">Save this key now. It won't be shown again.</p>
            <div className="mt-2 rounded-lg border border-[#1e1e1e] bg-[#0a0a0a] p-3 font-mono text-sm text-[#ededed] break-all">{createdKey}</div>
          </div>
        ) : (
          <form onSubmit={handleCreate} className="space-y-4">
            <div>
              <label className="block text-xs font-medium text-[#888888] mb-1.5">Name</label>
              <input
                type="text"
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="e.g., production-app"
                required
                className="h-9 w-full rounded-lg border border-[#262626] bg-[#141414] px-3 text-sm text-[#ededed] placeholder-[#555555] outline-none focus:border-accent/50 transition-colors"
              />
            </div>
            <div className="flex gap-4">
              <div className="flex-1">
                <label className="block text-xs font-medium text-[#888888] mb-1.5">Rate Limit (RPM)</label>
                <input
                  type="number"
                  value={rateLimit}
                  onChange={(e) => setRateLimit(e.target.value)}
                  placeholder="Unlimited"
                  min={1}
                  className="h-9 w-full rounded-lg border border-[#262626] bg-[#141414] px-3 text-sm text-[#ededed] placeholder-[#555555] outline-none focus:border-accent/50 transition-colors"
                />
              </div>
              <div className="flex-1">
                <label className="block text-xs font-medium text-[#888888] mb-1.5">Monthly Budget ($)</label>
                <input
                  type="number"
                  value={budget}
                  onChange={(e) => setBudget(e.target.value)}
                  placeholder="Unlimited"
                  min={0}
                  step={0.01}
                  className="h-9 w-full rounded-lg border border-[#262626] bg-[#141414] px-3 text-sm text-[#ededed] placeholder-[#555555] outline-none focus:border-accent/50 transition-colors"
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
