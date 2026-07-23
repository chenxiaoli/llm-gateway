import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Plus, KeyRound, TriangleAlert } from 'lucide-react';
import { motion } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import { useKeys, useCreateKey } from '../hooks/useKeys';
import { useModelFallbacks } from '../hooks/useModelFallbacks';
import { useAutoRouteConfigs } from '../hooks/useAutoRouteConfigs';
import { useCurrencyStore, formatCurrency } from '../stores/currency';
import { useReducedMotion } from '../hooks/useReducedMotion';
import { Button } from '../components/ui/Button';
import { Modal } from '../components/ui/Modal';
import { Badge } from '../components/ui/Badge';
import { ProgressBar } from '../components/ui/ProgressBar';
import { budgetBarColor, budgetUsedPct } from '../lib/budgetColor';
import { toast } from 'sonner';
import i18n from '../i18n';
import { useAuthStore } from '../stores/authStore';
import type { CreateKeyResponse } from '../types';

const EASE = [0.16, 1, 0.3, 1] as const;

export default function Keys() {
  const { t } = useTranslation();
  const symbol = useCurrencyStore((s) => s.symbol);
  const [page, setPage] = useState(1);
  const [pageSize] = useState(20);
  const { data, isLoading } = useKeys(page, pageSize);
  const createKeyMutation = useCreateKey();
  const slug = useAuthStore((s) => s.currentOrg?.slug);
  const navigate = useNavigate();
  const { data: fallbacks } = useModelFallbacks();
  const { data: autoRouteConfigs } = useAutoRouteConfigs();
  const reducedMotion = useReducedMotion();

  const [createOpen, setCreateOpen] = useState(false);
  const [createdKey, setCreatedKey] = useState<string | null>(null);
  const [name, setName] = useState('');
  const [rateLimit, setRateLimit] = useState('');
  const [budget, setBudget] = useState('');
  const [fallbackId, setFallbackId] = useState('');
  const [autoRouteId, setAutoRouteId] = useState('');

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    const result: CreateKeyResponse = await createKeyMutation.mutateAsync({
      name,
      rate_limit: rateLimit ? Number(rateLimit) : null,
      budget_monthly: budget ? Number(budget) : null,
      model_fallback_id: fallbackId || null,
      auto_route_id: autoRouteId || null,
    });
    setCreatedKey(result.key);
    setName('');
    setRateLimit('');
    setBudget('');
    setFallbackId('');
    setAutoRouteId('');
  };

  const copyKey = () => {
    if (createdKey) {
      navigator.clipboard.writeText(createdKey);
      toast.success(i18n.t('toasts.keyCopiedToClipboard'));
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
            {t('keys.title')}
          </h1>
          <p className="text-base text-base-content/50">
            {t('keys.description')}
          </p>
        </div>
        <Button icon={<Plus className="h-4 w-4" />} onClick={() => setCreateOpen(true)}>
          {t('keys.createKey')}
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
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('keys.table.name')}</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('keys.table.key')}</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('keys.table.status')}</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('keys.table.rateLimit')}</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('keys.table.monthlyBudget')}</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('keys.table.mtdThisMonth')}</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('keys.table.fallback')}</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('keys.table.created')}</th>
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
                      <button onClick={() => navigate(slug ? `/${slug}/keys/${key.id}` : '/login')} className="link link-primary text-sm font-medium">
                        {key.name}
                      </button>
                    </td>
                    <td className="font-mono text-sm text-base-content/40">
                      {key.key_prefix ? `${key.key_prefix}...` : '—'}
                    </td>
                    <td><Badge variant={key.enabled ? 'green' : 'red'}>{key.enabled ? t('keys.active') : t('keys.disabled')}</Badge></td>
                    <td className="font-mono text-sm text-base-content/55">{key.rate_limit ?? t('keys.unlimited')}</td>
                    <td className="font-mono text-sm text-base-content/55">{key.budget_monthly != null ? formatCurrency(key.budget_monthly, symbol, 2) : t('keys.unlimited')}</td>
                    <td className="font-mono text-sm text-base-content/55">
                      {(() => {
                        const UNITS_PER_USD = 100_000_000;
                        const budgetUnits =
                          key.budget_monthly !== null
                            ? Math.round(key.budget_monthly * UNITS_PER_USD)
                            : null;
                        const pct = budgetUsedPct(key.mtd_units ?? 0, budgetUnits);
                        const accruedUsd = (key.mtd_units ?? 0) / UNITS_PER_USD;
                        return (
                          <div className="flex items-center gap-2 min-w-[140px]">
                            <span className="tabular-nums">
                              {formatCurrency(accruedUsd, symbol, 2)}
                            </span>
                            <div className="flex-1">
                              <ProgressBar pct={pct ?? 0} colorClass={budgetBarColor(pct)} size="sm" />
                            </div>
                            <span className="text-xs text-base-content/45 tabular-nums w-10 text-right">
                              {pct !== null ? `${Math.round(pct)}%` : '—'}
                            </span>
                          </div>
                        );
                      })()}
                    </td>
                    <td className="text-sm text-base-content/55">{key.model_fallback_id ? (fallbacks?.find(f => f.id === key.model_fallback_id)?.name ?? '—') : t('keys.none')}</td>
                    <td className="font-mono text-sm text-base-content/50">{new Date(key.created_at).toLocaleDateString()}</td>
                  </motion.tr>
                ))}
                {(!data?.items?.length) && (
                  <tr>
                    <td colSpan={8} className="text-center py-16">
                      <div className="flex flex-col items-center gap-3">
                        <div className="w-12 h-12 rounded-xl flex items-center justify-center bg-base-200/60">
                          <KeyRound className="h-6 w-6 text-base-content/30" />
                        </div>
                        <p className="text-sm text-base-content/40">{t('keys.empty')}</p>
                        <Button variant="secondary" size="sm" onClick={() => setCreateOpen(true)}>
                          {t('keys.createFirst')}
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
          <span className="text-xs text-base-content/40">{t('keys.pagination.total', { count: data?.total ?? 0 })}</span>
          <div className="join">
            <Button variant="ghost" size="sm" disabled={page <= 1} onClick={() => setPage(page - 1)}>{t('keys.pagination.previous')}</Button>
            <span className="px-3 flex items-center text-sm text-base-content/50">{page} / {totalPages}</span>
            <Button variant="ghost" size="sm" disabled={page >= totalPages} onClick={() => setPage(page + 1)}>{t('keys.pagination.next')}</Button>
          </div>
        </motion.div>
      )}

      {/* Create Modal */}
      <Modal
        open={createOpen}
        onClose={() => { setCreateOpen(false); setCreatedKey(null); }}
        title={t('keys.createModal.title')}
        footer={createdKey ? (
          <div className="flex gap-2">
            <Button onClick={copyKey}>{t('keys.createModal.copyKey')}</Button>
            <Button variant="secondary" onClick={() => { setCreateOpen(false); setCreatedKey(null); }}>{t('keys.createModal.done')}</Button>
          </div>
        ) : undefined}
      >
        {createdKey ? (
          <div className="space-y-4">
            <div className="flex items-center gap-2">
              <TriangleAlert className="h-5 w-5 text-amber-500 shrink-0" />
              <p className="text-md font-semibold text-base-content">{t('keys.createModal.saveNow')}</p>
            </div>
            <p className="text-base text-base-content/70">
              {t('keys.createModal.securityNotice')} <strong className="text-base-content">{t('keys.createModal.securityWarning')}</strong>
            </p>
            <div className="rounded-xl border border-base-300 bg-base-200/60 p-4">
              <code className="font-mono text-base break-all select-all leading-relaxed">{createdKey}</code>
            </div>
          </div>
        ) : (
          <form onSubmit={handleCreate} className="space-y-4">
            <div className="form-control">
              <label className="label"><span className="label-text font-medium">{t('keys.form.name')}</span></label>
              <input
                type="text"
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder={t('keys.form.namePlaceholder')}
                required
                className="input input-bordered w-full"
              />
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div className="form-control">
                <label className="label"><span className="label-text font-medium">{t('keys.form.rateLimit')}</span></label>
                <input
                  type="number"
                  value={rateLimit}
                  onChange={(e) => setRateLimit(e.target.value)}
                  placeholder={t('keys.unlimited')}
                  min={1}
                  className="input input-bordered w-full"
                />
              </div>
              <div className="form-control">
                <label className="label"><span className="label-text font-medium">{t('keys.form.monthlyBudget')}</span></label>
                <input
                  type="number"
                  value={budget}
                  onChange={(e) => setBudget(e.target.value)}
                  placeholder={t('keys.unlimited')}
                  min={0}
                  step={0.01}
                  className="input input-bordered w-full"
                />
              </div>
            </div>
            <div className="form-control">
              <label className="label"><span className="label-text font-medium">{t('keys.form.modelFallback')}</span></label>
              <select value={fallbackId} onChange={(e) => setFallbackId(e.target.value)} className="select select-bordered w-full">
                <option value="">{t('keys.form.noneOption')}</option>
                {fallbacks?.map((fb) => (<option key={fb.id} value={fb.id}>{fb.name}</option>))}
              </select>
            </div>
            <div className="form-control">
              <label className="label"><span className="label-text font-medium">{t('keys.form.autoRouteConfig')}</span></label>
              <select value={autoRouteId} onChange={(e) => setAutoRouteId(e.target.value)} className="select select-bordered w-full">
                <option value="">{t('keys.form.noneOption')}</option>
                {autoRouteConfigs?.map((c) => (<option key={c.id} value={c.id}>{c.name}</option>))}
              </select>
            </div>
            <Button variant="primary" loading={createKeyMutation.isPending}>{t('common.create')}</Button>
          </form>
        )}
      </Modal>
    </div>
  );
}
