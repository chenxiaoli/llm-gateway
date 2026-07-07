import { useState } from 'react';
import { DollarSign } from 'lucide-react';
import { motion } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import { useAuthStore } from '../stores/authStore';
import { isAdminOrAbove } from '../lib/auth';
import { useCurrencyStore, formatCurrency } from '../stores/currency';
import { useMyBalance } from '../hooks/useAccounts';
import { useReducedMotion } from '../hooks/useReducedMotion';
import { Button } from '../components/ui/Button';
import { Badge } from '../components/ui/Badge';

const EASE = [0.16, 1, 0.3, 1] as const;

export default function Account() {
  const { t } = useTranslation();
  const user = useAuthStore((s) => s.user);
  const currentOrg = useAuthStore((s) => s.currentOrg);
  const symbol = useCurrencyStore((s) => s.symbol);
  const [page, setPage] = useState(1);
  const [pageSize] = useState(20);
  const reducedMotion = useReducedMotion();

  const { data, isLoading } = useMyBalance(page, pageSize);
  const transactions = data?.transactions;
  const totalPages = Math.ceil((transactions?.total ?? 0) / pageSize);

  // Phase 1: role badge reflects platform_admin (rare) or current org role.
  const isAdmin = isAdminOrAbove(user, currentOrg);

  const TX_TYPE_LABELS: Record<string, { label: string; color: 'green' | 'red' | 'blue' | 'purple' }> = {
    credit: { label: t('account.credit'), color: 'green' },
    debit: { label: t('account.debit'), color: 'red' },
    credit_adjustment: { label: t('account.adjustment'), color: 'blue' },
    debit_refund: { label: t('account.refund'), color: 'purple' },
  };

  return (
    <div className="px-6 pb-8">
      {/* Profile Header */}
      <motion.div
        initial={reducedMotion ? false : { opacity: 0, y: -12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={reducedMotion ? { duration: 0 } : { duration: 0.4, ease: EASE }}
        className="mb-8 pt-8"
      >
        <div className="flex items-center gap-5">
          <div className="w-16 h-16 rounded-2xl bg-primary/10 flex items-center justify-center shrink-0">
            <span className="text-2xl font-bold text-primary">{user?.username?.charAt(0).toUpperCase()}</span>
          </div>
          <div>
            <h1 className="text-3xl font-black tracking-tight text-base-content leading-none mb-1.5">
              {user?.username}
            </h1>
            <div className="flex items-center gap-2">
              <Badge variant={isAdmin ? 'green' : 'blue'}>
                {isAdmin ? t('account.administrator') : t('account.user')}
              </Badge>
            </div>
          </div>
        </div>
      </motion.div>

      {/* Balance Stats */}
      {data && (
        <motion.div
          initial={reducedMotion ? false : { opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={reducedMotion ? { duration: 0 } : { duration: 0.35, delay: 0.05, ease: EASE }}
          className="mb-6 grid grid-cols-3 gap-3 max-sm:grid-cols-1"
        >
          <div className="rounded-2xl border border-base-300/40 bg-base-100 p-5">
            <div className="text-xs font-semibold uppercase tracking-wider text-base-content/45 flex items-center gap-1.5 mb-2">
              <DollarSign className="h-4 w-4" />
              {t('account.balance')}
            </div>
            <div className="font-mono text-3xl font-bold tracking-tight">
              {formatCurrency(data.balance, symbol, 4)}
            </div>
          </div>
          <div className="rounded-2xl border border-base-300/40 bg-base-100 p-5">
            <div className="text-xs font-semibold uppercase tracking-wider text-base-content/45 mb-2">
              {t('account.threshold')}
            </div>
            <div className="font-mono text-3xl font-bold tracking-tight">{formatCurrency(data.threshold, symbol, 2)}</div>
          </div>
          <div className="rounded-2xl border border-base-300/40 bg-base-100 p-5">
            <div className="text-xs font-semibold uppercase tracking-wider text-base-content/45 mb-2">
              {t('account.status')}
            </div>
            <div className="mt-1">
              {data.balance <= data.threshold
                ? <Badge variant="amber">{t('account.lowBalance')}</Badge>
                : <Badge variant="green">{t('account.active')}</Badge>
              }
            </div>
          </div>
        </motion.div>
      )}

      {/* Transactions */}
      <motion.div
        initial={reducedMotion ? false : { opacity: 0, y: 12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={reducedMotion ? { duration: 0 } : { duration: 0.35, delay: 0.1, ease: EASE }}
        className="mb-8"
      >
        <h2 className="text-lg font-bold mb-3">{t('account.transactionHistory')}</h2>
        {isLoading ? (
          <div className="flex items-center justify-center py-12">
            <span className="loading loading-spinner loading-lg" />
          </div>
        ) : (
          <>
            <div className="overflow-x-auto rounded-2xl border border-base-300/40 bg-base-100">
              <table className="table table-sm">
                <thead>
                  <tr className="border-b border-base-300/40">
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('account.table.time')}</th>
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('account.table.type')}</th>
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45 text-right">{t('account.table.amount')}</th>
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45 text-right">{t('account.table.balanceAfter')}</th>
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('account.table.description')}</th>
                  </tr>
                </thead>
                <tbody>
                  {transactions?.items.map((tx) => {
                    const info = TX_TYPE_LABELS[tx.type] ?? { label: tx.type, color: 'gray' as const };
                    const isCredit = tx.type === 'credit' || tx.type === 'credit_adjustment';
                    return (
                      <tr key={tx.id} className="border-b border-base-200/40 hover:bg-base-200/20 transition-colors">
                        <td className="font-mono text-[13px] text-base-content/55">
                          {new Date(tx.created_at).toLocaleString()}
                        </td>
                        <td><Badge variant={info.color}>{info.label}</Badge></td>
                        <td className={`font-mono text-right text-sm ${isCredit ? 'text-green-500' : 'text-red-500'}`}>
                          {isCredit ? '+' : '-'}{formatCurrency(tx.amount, symbol, 4)}
                        </td>
                        <td className="font-mono text-right text-sm text-base-content/55">{formatCurrency(tx.balance_after, symbol, 4)}</td>
                        <td className="text-sm text-base-content/55">{tx.description ?? '-'}</td>
                      </tr>
                    );
                  })}
                  {transactions?.items.length === 0 && (
                    <tr>
                      <td colSpan={5} className="text-center py-8 text-base-content/40">
                        {t('account.noTransactions')}
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>

            {totalPages > 1 && (
              <div className="mt-4 flex items-center justify-between text-sm">
                <span className="text-xs text-base-content/40">{t('account.pagination.total', { count: transactions?.total ?? 0 })}</span>
                <div className="join">
                  <Button variant="ghost" size="sm" disabled={page <= 1} onClick={() => setPage(page - 1)}>
                    {t('account.pagination.previous')}
                  </Button>
                  <span className="px-3 flex items-center text-sm text-base-content/60">
                    {page} / {totalPages}
                  </span>
                  <Button variant="ghost" size="sm" disabled={page >= totalPages} onClick={() => setPage(page + 1)}>
                    {t('account.pagination.next')}
                  </Button>
                </div>
              </div>
            )}
          </>
        )}
      </motion.div>

    </div>
  );
}
