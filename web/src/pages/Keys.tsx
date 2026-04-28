import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Plus, KeyRound } from 'lucide-react';
import { motion } from 'framer-motion';
import { useKeys, useCreateKey } from '../hooks/useKeys';
import { useModelFallbacks } from '../hooks/useModelFallbacks';
import { useReducedMotion } from '../hooks/useReducedMotion';
import { Button } from '../components/ui/Button';
import { Modal } from '../components/ui/Modal';
import { Badge } from '../components/ui/Badge';
import { toast } from 'sonner';
import type { CreateKeyResponse } from '../types';

const EASE = [0.16, 1, 0.3, 1] as const;

export default function Keys() {
  const [page, setPage] = useState(1);
  const [pageSize] = useState(20);
  const { data, isLoading } = useKeys(page, pageSize);
  const createKeyMutation = useCreateKey();
  const navigate = useNavigate();
  const { data: fallbacks } = useModelFallbacks();
  const reducedMotion = useReducedMotion();

  const [createOpen, setCreateOpen] = useState(false);
  const [createdKey, setCreatedKey] = useState<string | null>(null);
  const [name, setName] = useState('');
  const [rateLimit, setRateLimit] = useState('');
  const [budget, setBudget] = useState('');
  const [fallbackId, setFallbackId] = useState('');

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    const result: CreateKeyResponse = await createKeyMutation.mutateAsync({
      name,
      rate_limit: rateLimit ? Number(rateLimit) : null,
      budget_monthly: budget ? Number(budget) : null,
      model_fallback_id: fallbackId || null,
    });
    setCreatedKey(result.key);
    setName('');
    setRateLimit('');
    setBudget('');
    setFallbackId('');
  };

  const copyKey = () => {
    if (createdKey) {
      navigator.clipboard.writeText(createdKey);
      toast.success('Key copied to clipboard');
    }
  };

  const totalPages = Math.ceil((data?.total ?? 0) / pageSize);

  return (
    <div className="px-6 pb-8">
      {/* Header */}
      <motion.div
        initial={reducedMotion ? false : { opacity: 0, y: -12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={reducedMotion ? { duration: 0 } : { duration: 0.4, ease: EASE }}
        className="mb-8 pt-8 flex items-end justify-between gap-6"
      >
        <div>
          <h1 className="text-3xl font-black tracking-tight text-base-content leading-none mb-1">
            API Keys
          </h1>
          <p className="text-base text-base-content/50">
            Manage access keys for API authentication
          </p>
        </div>
        <Button icon={<Plus className="h-4 w-4" />} onClick={() => setCreateOpen(true)}>
          Create Key
        </Button>
      </motion.div>

      {/* Table */}
      {isLoading ? (
        <motion.div
          initial={reducedMotion ? false : { opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={reducedMotion ? { duration: 0 } : { duration: 0.35, delay: 0.05, ease: EASE }}
          className="rounded-2xl border border-base-300/40 bg-base-100 overflow-hidden"
        >
          <div className="p-5 space-y-3">
            {[...Array(5)].map((_, i) => (
              <div key={i} className="h-10 bg-base-200/40 rounded-lg animate-pulse" />
            ))}
          </div>
        </motion.div>
      ) : (
        <motion.div
          initial={reducedMotion ? false : { opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={reducedMotion ? { duration: 0 } : { duration: 0.35, delay: 0.05, ease: EASE }}
          className="rounded-2xl border border-base-300/40 bg-base-100 overflow-hidden"
        >
          <div className="overflow-x-auto">
            <table className="table table-sm">
              <thead>
                <tr className="border-b border-base-300/40">
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">Name</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">Status</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">Rate Limit (RPM)</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">Monthly Budget</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">Fallback</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">Created</th>
                </tr>
              </thead>
              <tbody>
                {data?.items?.map((key, index) => (
                  <motion.tr
                    key={key.id}
                    initial={reducedMotion ? false : { opacity: 0 }}
                    animate={{ opacity: 1 }}
                    transition={reducedMotion ? { duration: 0 } : { duration: 0.25, delay: 0.1 + index * 0.03, ease: EASE }}
                    className="border-b border-base-200/40 hover:bg-base-200/20 transition-colors"
                  >
                    <td>
                      <button onClick={() => navigate(`/console/keys/${key.id}`)} className="link link-primary text-sm font-medium">
                        {key.name}
                      </button>
                    </td>
                    <td><Badge variant={key.enabled ? 'green' : 'red'}>{key.enabled ? 'Active' : 'Disabled'}</Badge></td>
                    <td className="font-mono text-sm text-base-content/55">{key.rate_limit ?? 'Unlimited'}</td>
                    <td className="font-mono text-sm text-base-content/55">{key.budget_monthly != null ? `$${key.budget_monthly.toFixed(2)}` : 'Unlimited'}</td>
                    <td className="text-sm text-base-content/55">{key.model_fallback_id ? (fallbacks?.find(f => f.id === key.model_fallback_id)?.name ?? '—') : 'None'}</td>
                    <td className="font-mono text-sm text-base-content/50">{new Date(key.created_at).toLocaleDateString()}</td>
                  </motion.tr>
                ))}
                {(!data?.items?.length) && (
                  <tr>
                    <td colSpan={6} className="text-center py-16">
                      <div className="flex flex-col items-center gap-3">
                        <div className="w-12 h-12 rounded-xl flex items-center justify-center bg-base-200/60">
                          <KeyRound className="h-6 w-6 text-base-content/30" />
                        </div>
                        <p className="text-sm text-base-content/40">No API keys yet</p>
                        <Button variant="secondary" size="sm" onClick={() => setCreateOpen(true)}>
                          Create your first key
                        </Button>
                      </div>
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </motion.div>
      )}

      {/* Pagination */}
      {totalPages > 1 && (
        <motion.div
          initial={reducedMotion ? false : { opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={reducedMotion ? { duration: 0 } : { duration: 0.3, delay: 0.2, ease: EASE }}
          className="mt-4 flex items-center justify-between text-sm"
        >
          <span className="text-xs text-base-content/40">Total {data?.total ?? 0}</span>
          <div className="join">
            <Button variant="ghost" size="sm" disabled={page <= 1} onClick={() => setPage(page - 1)}>Previous</Button>
            <span className="px-3 flex items-center text-sm text-base-content/50">{page} / {totalPages}</span>
            <Button variant="ghost" size="sm" disabled={page >= totalPages} onClick={() => setPage(page + 1)}>Next</Button>
          </div>
        </motion.div>
      )}

      {/* Create Modal */}
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
            <div className="form-control">
              <label className="label"><span className="label-text font-medium">Model Fallback</span></label>
              <select value={fallbackId} onChange={(e) => setFallbackId(e.target.value)} className="select select-bordered w-full">
                <option value="">None</option>
                {fallbacks?.map((fb) => (<option key={fb.id} value={fb.id}>{fb.name}</option>))}
              </select>
            </div>
            <Button variant="primary" loading={createKeyMutation.isPending}>Create</Button>
          </form>
        )}
      </Modal>
    </div>
  );
}
