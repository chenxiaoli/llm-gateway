import { useState, useEffect } from 'react';
import { createPortal } from 'react-dom';
import { Wallet, X, DollarSign, MessageSquare, Zap } from 'lucide-react';
import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from 'recharts';
import { motion, AnimatePresence } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import { useUsers, useUpdateUser, useDeleteUser } from '../hooks/useUsers';
import { useUserBalance, useRechargeUser, useAdjustUser } from '../hooks/useAccounts';
import { useUsage, useUsageSummary } from '../hooks/useUsage';
import { useQueryClient } from '@tanstack/react-query';
import { useReducedMotion } from '../hooks/useReducedMotion';
import { Button } from '../components/ui/Button';
import { Badge } from '../components/ui/Badge';
import { Select } from '../components/ui/Select';
import { ConfirmDialog } from '../components/ui/ConfirmDialog';
import { Modal } from '../components/ui/Modal';

const EASE = [0.16, 1, 0.3, 1] as const;

// ── Shared Drawer Shell ───────────────────────────────────────────────────────

function DrawerShell({ title, isOpen, onClose, width = 560, children }: {
  title: string;
  isOpen: boolean;
  onClose: () => void;
  width?: number;
  children: React.ReactNode;
}) {
  const reducedMotion = useReducedMotion();

  useEffect(() => {
    if (isOpen) {
      document.body.style.overflow = 'hidden';
      return () => { document.body.style.overflow = ''; };
    }
  }, [isOpen]);

  if (!isOpen) return null;

  const drawer = (
    <AnimatePresence>
      {isOpen && (
        <>
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={reducedMotion ? { duration: 0 } : { duration: 0.2 }}
            className="fixed inset-0 z-[110] bg-black/50"
            onClick={onClose}
          />
          <motion.div
            initial={reducedMotion ? { x: 0 } : { x: '100%' }}
            animate={{ x: 0 }}
            exit={reducedMotion ? { x: 0 } : { x: '100%' }}
            transition={reducedMotion ? { duration: 0 } : { duration: 0.3, ease: EASE }}
            className="fixed right-0 top-0 bottom-0 z-[120] max-w-[100vw] flex flex-col bg-base-100 border-l border-base-300/60"
            style={{ width: `${width}px`, boxShadow: '-8px 0 32px rgba(0, 0, 0, 0.3)' }}
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center justify-between px-6 py-5 shrink-0 border-b border-base-300/40">
              <h3 className="text-lg font-bold">{title}</h3>
              <button
                onClick={onClose}
                aria-label="Close"
                className="w-8 h-8 flex items-center justify-center rounded-lg cursor-pointer text-base-content/40 hover:text-base-content hover:bg-base-200/60 transition-colors"
              >
                <X className="h-4 w-4" />
              </button>
            </div>
            <div className="flex-1 overflow-y-auto px-6 py-5">
              {children}
            </div>
          </motion.div>
        </>
      )}
    </AnimatePresence>
  );

  return createPortal(drawer, document.body);
}

// ── User Detail Drawer ────────────────────────────────────────────────────────

function UserDrawer({ userId, onClose }: { userId: string | null; onClose: () => void }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const { data, isLoading } = useUserBalance(userId ?? '', 1, 10);
  const rechargeMutation = useRechargeUser();
  const adjustMutation = useAdjustUser();

  const [rechargeOpen, setRechargeOpen] = useState(false);
  const [adjustOpen, setAdjustOpen] = useState(false);
  const [rechargeAmount, setRechargeAmount] = useState('');
  const [adjustAmount, setAdjustAmount] = useState('');
  const [adjustType, setAdjustType] = useState<'credit_adjustment' | 'debit_refund'>('credit_adjustment');
  const [description, setDescription] = useState('');

  const account = data?.account;
  const transactions = data?.transactions?.items ?? [];

  const TX_TYPE_LABELS: Record<string, { label: string; color: 'green' | 'red' | 'blue' | 'purple' }> = {
    credit: { label: t('users.drawer.credit'), color: 'green' },
    debit: { label: t('users.drawer.debit'), color: 'red' },
    credit_adjustment: { label: t('users.drawer.adjustment'), color: 'blue' },
    debit_refund: { label: t('users.drawer.refund'), color: 'purple' },
  };

  const handleRecharge = () => {
    const amount = parseFloat(rechargeAmount);
    if (!amount || amount <= 0 || !userId) return;
    rechargeMutation.mutate(
      { userId, data: { type: 'credit', amount, description: description || t('users.drawer.credit') } },
      {
        onSuccess: () => {
          setRechargeOpen(false);
          setRechargeAmount('');
          setDescription('');
          queryClient.invalidateQueries({ queryKey: ['users'] });
        },
      }
    );
  };

  const handleAdjust = () => {
    const amount = parseFloat(adjustAmount);
    if (!amount || amount <= 0 || !userId) return;
    adjustMutation.mutate(
      { userId, data: { type: adjustType, amount, description: description || t('users.drawer.adjustment') } },
      {
        onSuccess: () => {
          setAdjustOpen(false);
          setAdjustAmount('');
          setDescription('');
          queryClient.invalidateQueries({ queryKey: ['users'] });
        },
      }
    );
  };

  const isOpen = userId !== null;

  return (
    <DrawerShell title={t('users.drawer.title')} isOpen={isOpen} onClose={onClose} width={480}>
      {isLoading ? (
        <div className="flex items-center justify-center py-12">
          <span className="loading loading-spinner loading-lg" />
        </div>
      ) : (
        <div className="space-y-6">
          {account && (
            <div className="rounded-2xl border border-base-300/40 bg-base-100 p-5">
              <div className="flex items-center justify-between mb-4">
                <span className="text-xs font-semibold uppercase tracking-wider text-base-content/50">{t('users.drawer.accountBalance')}</span>
                <div className={`w-8 h-8 rounded-lg flex items-center justify-center ${
                  account.balance <= account.threshold ? 'bg-amber-500/10' : 'bg-primary/10'
                }`}>
                  <Wallet className={`h-4 w-4 ${account.balance <= account.threshold ? 'text-amber-500' : 'text-primary'}`} />
                </div>
              </div>
              <div className="font-mono text-3xl font-bold tracking-tight mb-3">
                ${account.balance.toFixed(4)}
                <span className="text-sm text-base-content/40 ml-2 font-normal">{account.currency}</span>
              </div>
              <div className="flex items-center gap-3">
                <span className="text-xs text-base-content/40">{t('users.drawer.threshold')}: ${account.threshold.toFixed(2)}</span>
                {account.balance <= account.threshold && <Badge variant="amber">{t('users.drawer.lowBalance')}</Badge>}
              </div>
            </div>
          )}

          <div className="flex gap-2">
            <Button size="sm" className="flex-1" onClick={() => setRechargeOpen(true)}>{t('users.drawer.recharge')}</Button>
            <Button variant="secondary" size="sm" className="flex-1" onClick={() => setAdjustOpen(true)}>{t('users.drawer.adjust')}</Button>
          </div>

          <div>
            <h4 className="text-sm font-bold text-base-content/70 mb-3">{t('users.drawer.recentTransactions')}</h4>
            {transactions.length === 0 ? (
              <div className="text-center py-8 text-base-content/40 text-sm">{t('users.drawer.noTransactions')}</div>
            ) : (
              <div className="space-y-2">
                {transactions.map((tx) => {
                  const info = TX_TYPE_LABELS[tx.type] ?? { label: tx.type, color: 'gray' as const };
                  const isCredit = tx.type === 'credit' || tx.type === 'credit_adjustment';
                  return (
                    <div key={tx.id} className="flex items-center justify-between py-2.5 border-b border-base-200/40 last:border-0">
                      <div className="min-w-0 flex-1 mr-3">
                        <div className="flex items-center gap-2 mb-0.5">
                          <Badge variant={info.color}>{info.label}</Badge>
                        </div>
                        <div className="text-xs text-base-content/40 truncate">{tx.description ?? '-'}</div>
                      </div>
                      <div className="text-right shrink-0">
                        <div className={`font-mono text-sm font-medium ${isCredit ? 'text-green-500' : 'text-red-500'}`}>
                          {isCredit ? '+' : '-'}${tx.amount.toFixed(4)}
                        </div>
                        <div className="text-[11px] text-base-content/30">{new Date(tx.created_at).toLocaleDateString()}</div>
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        </div>
      )}

      {/* Recharge Modal */}
      <Modal open={rechargeOpen} onClose={() => setRechargeOpen(false)} title={t('users.rechargeModal.title')}>
        <div className="space-y-4">
          <div>
            <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-1.5 block">{t('users.rechargeModal.amount')}</label>
            <input
              type="number"
              value={rechargeAmount}
              onChange={(e) => setRechargeAmount(e.target.value)}
              placeholder="0.00"
              step="0.01"
              min="0"
              className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm text-base-content focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
            />
          </div>
          <div>
            <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-1.5 block">{t('users.rechargeModal.descriptionLabel')}</label>
            <input
              type="text"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder={t('users.rechargeModal.descriptionPlaceholder')}
              className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm text-base-content focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
            />
          </div>
          <div className="flex justify-end gap-2 pt-2">
            <Button variant="ghost" onClick={() => setRechargeOpen(false)}>{t('common.cancel')}</Button>
            <Button onClick={handleRecharge} loading={rechargeMutation.isPending}>{t('users.rechargeModal.confirmRecharge')}</Button>
          </div>
        </div>
      </Modal>

      {/* Adjust Modal */}
      <Modal open={adjustOpen} onClose={() => setAdjustOpen(false)} title={t('users.adjustModal.title')}>
        <div className="space-y-4">
          <div>
            <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-1.5 block">{t('users.adjustModal.type')}</label>
            <select
              value={adjustType}
              onChange={(e) => setAdjustType(e.target.value as 'credit_adjustment' | 'debit_refund')}
              className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm text-base-content focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors cursor-pointer"
            >
              <option value="credit_adjustment">{t('users.adjustModal.creditAdjustment')}</option>
              <option value="debit_refund">{t('users.adjustModal.debitRefund')}</option>
            </select>
          </div>
          <div>
            <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-1.5 block">{t('users.adjustModal.amount')}</label>
            <input
              type="number"
              value={adjustAmount}
              onChange={(e) => setAdjustAmount(e.target.value)}
              placeholder="0.00"
              step="0.01"
              min="0"
              className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm text-base-content focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
            />
          </div>
          <div>
            <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-1.5 block">{t('users.adjustModal.descriptionLabel')}</label>
            <input
              type="text"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder={t('users.adjustModal.descriptionPlaceholder')}
              className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm text-base-content focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
            />
          </div>
          <div className="flex justify-end gap-2 pt-2">
            <Button variant="ghost" onClick={() => setAdjustOpen(false)}>{t('common.cancel')}</Button>
            <Button onClick={handleAdjust} loading={adjustMutation.isPending}>{t('users.adjustModal.confirmAdjustment')}</Button>
          </div>
        </div>
      </Modal>
    </DrawerShell>
  );
}

// ── Usage Drawer ──────────────────────────────────────────────────────────────

function UsageDrawer({ userId, onClose }: { userId: string | null; onClose: () => void }) {
  const { t } = useTranslation();
  const [page, setPage] = useState(1);
  const [pageSize] = useState(20);
  const usageFilter = userId ? { user_id: userId } : {};
  const { data: usageData, isLoading: usageLoading } = useUsage(usageFilter, page, pageSize);
  const { data: usageSummary, isLoading: summaryLoading } = useUsageSummary(usageFilter);

  const isOpen = userId !== null;

  // Reset page when switching users
  useEffect(() => { setPage(1); }, [userId]);

  const items = usageData?.items ?? [];
  const totalPages = Math.ceil((usageData?.total ?? 0) / pageSize);
  const summary = usageSummary ?? [];

  const totals = summary.reduce<{ cost: number; requests: number; inputTokens: number; cacheTokens: number; outputTokens: number }>(
    (acc, r) => ({
      cost: acc.cost + r.total_cost,
      requests: acc.requests + r.request_count,
      inputTokens: acc.inputTokens + r.total_input_tokens,
      cacheTokens: acc.cacheTokens + r.total_cache_read_tokens,
      outputTokens: acc.outputTokens + r.total_output_tokens,
    }),
    { cost: 0, requests: 0, inputTokens: 0, cacheTokens: 0, outputTokens: 0 }
  );

  const chartData = summary.map((r) => ({
    model: r.model_name,
    input: r.total_input_tokens,
    cache: r.total_cache_read_tokens,
    output: r.total_output_tokens,
  }));

  return (
    <DrawerShell title={t('users.usageDrawer.title')} isOpen={isOpen} onClose={onClose} width={800}>
      {(usageLoading || summaryLoading) ? (
        <div className="flex items-center justify-center py-12">
          <span className="loading loading-spinner loading-lg" />
        </div>
      ) : totals.requests === 0 ? (
        <div className="text-center py-16 text-base-content/40 text-sm">{t('users.usageDrawer.noUsage')}</div>
      ) : (
        <div className="space-y-6">
          {/* Stat Cards */}
          <div className="grid grid-cols-2 gap-3">
            <div className="rounded-2xl border border-base-300/40 bg-base-100 p-4">
              <div className="flex items-center gap-2 mb-2">
                <DollarSign className="h-4 w-4 text-emerald-400" />
                <span className="text-[11px] font-semibold uppercase tracking-wider text-base-content/50">{t('users.usageDrawer.totalCost')}</span>
              </div>
              <div className="font-mono text-2xl font-bold">${totals.cost.toFixed(4)}</div>
            </div>
            <div className="rounded-2xl border border-base-300/40 bg-base-100 p-4">
              <div className="flex items-center gap-2 mb-2">
                <MessageSquare className="h-4 w-4 text-blue-400" />
                <span className="text-[11px] font-semibold uppercase tracking-wider text-base-content/50">{t('users.usageDrawer.requests')}</span>
              </div>
              <div className="font-mono text-2xl font-bold">{totals.requests.toLocaleString()}</div>
            </div>
            <div className="rounded-2xl border border-base-300/40 bg-base-100 p-4">
              <div className="flex items-center gap-2 mb-2">
                <Zap className="h-4 w-4 text-amber-400" />
                <span className="text-[11px] font-semibold uppercase tracking-wider text-base-content/50">{t('users.usageDrawer.inputTokens')}</span>
              </div>
              <div className="font-mono text-2xl font-bold">{totals.inputTokens.toLocaleString()}</div>
            </div>
            <div className="rounded-2xl border border-base-300/40 bg-base-100 p-4">
              <div className="flex items-center gap-2 mb-2">
                <Zap className="h-4 w-4 text-rose-400" />
                <span className="text-[11px] font-semibold uppercase tracking-wider text-base-content/50">{t('users.usageDrawer.outputTokens')}</span>
              </div>
              <div className="font-mono text-2xl font-bold">{totals.outputTokens.toLocaleString()}</div>
            </div>
          </div>

          {/* Chart */}
          {chartData.length > 0 && (
            <div className="rounded-2xl border border-base-300/40 bg-base-100 p-5">
              <h4 className="text-sm font-bold text-base-content/70 mb-4">{t('users.usageDrawer.tokenUsageByModel')}</h4>
              <ResponsiveContainer width="100%" height={260}>
                <BarChart data={chartData}>
                  <CartesianGrid strokeDasharray="3 3" stroke="var(--color-base-300)" opacity={0.4} />
                  <XAxis dataKey="model" tick={{ fill: 'var(--color-base-content)', fontSize: 11, opacity: 0.5 }} />
                  <YAxis tick={{ fill: 'var(--color-base-content)', fontSize: 11, opacity: 0.5 }} />
                  <Tooltip
                    contentStyle={{ background: 'var(--color-base-200)', border: '1px solid var(--color-base-300)', borderRadius: 12, fontSize: 12 }}
                    formatter={(value: number, name: string) => [value.toLocaleString(), name.charAt(0).toUpperCase() + name.slice(1)]}
                  />
                  <Bar dataKey="input" stackId="a" fill="var(--color-primary)" name="Input" />
                  <Bar dataKey="cache" stackId="a" fill="#60a5fa" name="Cache" />
                  <Bar dataKey="output" stackId="a" fill="var(--color-secondary)" name="Output" radius={[4, 4, 0, 0]} />
                </BarChart>
              </ResponsiveContainer>
            </div>
          )}

          {/* Usage Table */}
          <div className="rounded-2xl border border-base-300/40 bg-base-100 overflow-hidden">
            <table className="table table-sm">
              <thead>
                <tr className="border-b border-base-300/40">
                  <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/45">{t('users.usageDrawer.table.time')}</th>
                  <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/45">{t('users.usageDrawer.table.model')}</th>
                  <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/45">{t('users.usageDrawer.table.protocol')}</th>
                  <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/45 text-right">{t('users.usageDrawer.table.input')}</th>
                  <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/45 text-right">{t('users.usageDrawer.table.output')}</th>
                  <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/45 text-right">{t('users.usageDrawer.table.cost')}</th>
                </tr>
              </thead>
              <tbody>
                {items.map((item) => (
                  <tr key={item.id} className="border-b border-base-200/40 hover:bg-base-200/20 transition-colors">
                    <td className="font-mono text-[12px] text-base-content/55">
                      {new Date(item.created_at).toLocaleString()}
                    </td>
                    <td className="font-mono text-xs font-medium">{item.model_name}</td>
                    <td><Badge variant={item.protocol === 'openai' ? 'blue' : 'purple'}>{item.protocol}</Badge></td>
                    <td className="font-mono text-xs text-right text-base-content/55">{(item.input_tokens ?? 0).toLocaleString()}</td>
                    <td className="font-mono text-xs text-right text-base-content/55">{(item.output_tokens ?? 0).toLocaleString()}</td>
                    <td className="font-mono text-xs text-right">${item.cost.toFixed(6)}</td>
                  </tr>
                ))}
                {items.length === 0 && (
                  <tr>
                    <td colSpan={6} className="text-center py-8 text-base-content/40 text-sm">
                      {t('users.usageDrawer.noUsageRecords')}
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>

          {/* Pagination */}
          {totalPages > 1 && (
            <div className="flex items-center justify-between text-sm">
              <span className="text-xs text-base-content/40">{t('users.usageDrawer.pagination.total', { count: usageData?.total ?? 0 })}</span>
              <div className="join">
                <Button variant="ghost" size="sm" disabled={page <= 1} onClick={() => setPage(page - 1)}>
                  {t('users.usageDrawer.pagination.previous')}
                </Button>
                <span className="px-3 flex items-center text-sm text-base-content/60">
                  {page} / {totalPages}
                </span>
                <Button variant="ghost" size="sm" disabled={page >= totalPages} onClick={() => setPage(page + 1)}>
                  {t('users.usageDrawer.pagination.next')}
                </Button>
              </div>
            </div>
          )}
        </div>
      )}
    </DrawerShell>
  );
}

// ── Main Page ─────────────────────────────────────────────────────────────────

export default function Users() {
  const { t } = useTranslation();
  const [page, setPage] = useState(1);
  const [pageSize] = useState(20);
  const reducedMotion = useReducedMotion();
  const { data, isLoading } = useUsers(page, pageSize);
  const updateMutation = useUpdateUser();
  const deleteMutation = useDeleteUser();

  const [drawerUserId, setDrawerUserId] = useState<string | null>(null);
  const [usageUserId, setUsageUserId] = useState<string | null>(null);

  const totalPages = Math.ceil((data?.total ?? 0) / pageSize);

  return (
    <div className="px-6 pb-8">
      {/* Header */}
      <motion.div
        initial={reducedMotion ? false : { opacity: 0, y: -12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={reducedMotion ? { duration: 0 } : { duration: 0.4, ease: EASE }}
        className="mb-8 pt-8"
      >
        <h1 className="text-3xl font-black tracking-tight text-base-content leading-none mb-1">
          {t('users.title')}
        </h1>
        <p className="text-base text-base-content/50">
          {t('users.description')}
        </p>
      </motion.div>

      {isLoading ? (
        <div className="flex items-center justify-center py-12">
          <span className="loading loading-spinner loading-lg" />
        </div>
      ) : (
        <>
          <motion.div
            initial={reducedMotion ? false : { opacity: 0, y: 12 }}
            animate={{ opacity: 1, y: 0 }}
            transition={reducedMotion ? { duration: 0 } : { duration: 0.35, delay: 0.05, ease: EASE }}
            className="overflow-x-auto rounded-2xl border border-base-300/40 bg-base-100"
          >
            <table className="table table-sm">
              <thead>
                <tr className="border-b border-base-300/40">
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('users.table.username')}</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('users.table.role')}</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('users.table.status')}</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45 text-right">{t('users.table.balance')}</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('users.table.created')}</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('users.table.actions')}</th>
                </tr>
              </thead>
              <tbody>
                {data?.items?.map((user) => (
                  <tr key={user.id} className="border-b border-base-200/40 hover:bg-base-200/20 transition-colors">
                    <td>
                      <button
                        className="font-medium text-left cursor-pointer hover:text-accent transition-colors"
                        onClick={() => setDrawerUserId(user.id)}
                      >
                        {user.username}
                      </button>
                    </td>
                    <td>
                      <Select
                        value={user.role}
                        size="sm"
                        onChange={(value) => updateMutation.mutate({ id: user.id, input: { role: value as 'admin' | 'user' } })}
                        options={[{ value: 'admin', label: t('users.roleAdmin') }, { value: 'user', label: t('users.roleUser') }]}
                      />
                    </td>
                    <td>
                      <button
                        className="cursor-pointer"
                        onClick={() => updateMutation.mutate({ id: user.id, input: { enabled: !user.enabled } })}
                      >
                        <Badge variant={user.enabled ? 'green' : 'red'}>{user.enabled ? t('users.enabled') : t('users.disabled')}</Badge>
                      </button>
                    </td>
                    <td className="text-right">
                      <button
                        className="cursor-pointer font-mono text-sm"
                        onClick={() => setDrawerUserId(user.id)}
                      >
                        <span className={user.balance <= user.threshold ? 'text-amber-500' : 'text-base-content/70'}>
                          ${user.balance.toFixed(2)}
                        </span>
                      </button>
                    </td>
                    <td className="font-mono text-sm text-base-content/55">{new Date(user.created_at).toLocaleDateString()}</td>
                    <td>
                      <div className="flex items-center gap-1">
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => setDrawerUserId(user.id)}
                        >
                          {t('users.detail')}
                        </Button>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => setUsageUserId(user.id)}
                        >
                          {t('users.usage')}
                        </Button>
                        <ConfirmDialog title={t('users.deleteUser')} onConfirm={() => deleteMutation.mutate(user.id)} okText={t('common.delete')}>
                          <Button variant="danger" size="sm">{t('common.delete')}</Button>
                        </ConfirmDialog>
                      </div>
                    </td>
                  </tr>
                ))}
                {data?.items?.length === 0 && (
                  <tr>
                    <td colSpan={6} className="text-center py-12 text-base-content/40 text-sm">
                      {t('users.noUsers')}
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </motion.div>

          {totalPages > 1 && (
            <div className="mt-4 flex items-center justify-between text-sm">
              <span className="text-xs text-base-content/40">{t('users.pagination.total', { count: data?.total ?? 0 })}</span>
              <div className="join">
                <Button variant="ghost" size="sm" disabled={page <= 1} onClick={() => setPage(page - 1)}>
                  {t('users.pagination.previous')}
                </Button>
                <span className="px-3 flex items-center text-sm text-base-content/60">
                  {page} / {totalPages}
                </span>
                <Button variant="ghost" size="sm" disabled={page >= totalPages} onClick={() => setPage(page + 1)}>
                  {t('users.pagination.next')}
                </Button>
              </div>
            </div>
          )}
        </>
      )}

      <UserDrawer
        userId={drawerUserId}
        onClose={() => setDrawerUserId(null)}
      />
      <UsageDrawer
        userId={usageUserId}
        onClose={() => setUsageUserId(null)}
      />
    </div>
  );
}
