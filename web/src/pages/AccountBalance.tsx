import { useState } from 'react';
import { useParams } from 'react-router-dom';
import { DollarSign } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useUserBalance, useRechargeUser, useAdjustUser } from '../hooks/useAccounts';
import { Button } from '../components/ui/Button';
import { Badge } from '../components/ui/Badge';
import { Modal } from '../components/ui/Modal';

export default function AccountBalance() {
  const { t } = useTranslation();
  const { userId } = useParams<{ userId: string }>();
  const [page, setPage] = useState(1);
  const [pageSize] = useState(20);
  const [rechargeOpen, setRechargeOpen] = useState(false);
  const [adjustOpen, setAdjustOpen] = useState(false);
  const [rechargeAmount, setRechargeAmount] = useState('');
  const [adjustAmount, setAdjustAmount] = useState('');
  const [adjustType, setAdjustType] = useState<'credit_adjustment' | 'debit_refund'>('credit_adjustment');
  const [description, setDescription] = useState('');

  const { data, isLoading } = useUserBalance(userId ?? '', page, pageSize);
  const rechargeMutation = useRechargeUser();
  const adjustMutation = useAdjustUser();

  const account = data?.account;
  const transactions = data?.transactions;
  const totalPages = Math.ceil((transactions?.total ?? 0) / pageSize);

  const TX_TYPE_LABELS: Record<string, { label: string; color: 'green' | 'red' | 'blue' | 'purple' }> = {
    credit: { label: t('accountBalance.credit'), color: 'green' },
    debit: { label: t('accountBalance.debit'), color: 'red' },
    credit_adjustment: { label: t('accountBalance.adjustment'), color: 'blue' },
    debit_refund: { label: t('accountBalance.refund'), color: 'purple' },
  };

  const handleRecharge = () => {
    const amount = parseFloat(rechargeAmount);
    if (!amount || amount <= 0 || !userId) return;
    rechargeMutation.mutate(
      {
        userId,
        data: { type: 'credit', amount, description: description || t('accountBalance.credit') },
      },
      {
        onSuccess: () => {
          setRechargeOpen(false);
          setRechargeAmount('');
          setDescription('');
        },
      }
    );
  };

  const handleAdjust = () => {
    const amount = parseFloat(adjustAmount);
    if (!amount || amount <= 0 || !userId) return;
    adjustMutation.mutate(
      {
        userId,
        data: { type: adjustType, amount, description: description || t('accountBalance.adjustment') },
      },
      {
        onSuccess: () => {
          setAdjustOpen(false);
          setAdjustAmount('');
          setDescription('');
        },
      }
    );
  };

  return (
    <div>
      <div className="mb-6">
        <h1 className="text-2xl font-bold">{t('accountBalance.title')}</h1>
      </div>

      {/* Balance Header */}
      {account && (
        <div className="mb-6 grid grid-cols-3 gap-4 max-lg:grid-cols-2">
          <div className="stat bg-base-100 rounded-box p-4 shadow-sm">
            <div className="stat-title text-base-content/50">
              <DollarSign className="h-4 w-4 inline mr-1" />
              {t('accountBalance.balance')}
            </div>
            <div className="stat-value text-3xl font-mono">${account.balance.toFixed(4)}</div>
          </div>
          <div className="stat bg-base-100 rounded-box p-4 shadow-sm">
            <div className="stat-title text-base-content/50">{t('accountBalance.threshold')}</div>
            <div className="stat-value text-2xl font-mono">${account.threshold.toFixed(2)}</div>
          </div>
          <div className="stat bg-base-100 rounded-box p-4 shadow-sm">
            <div className="stat-title text-base-content/50">{t('accountBalance.currency')}</div>
            <div className="stat-value text-2xl">{account.currency}</div>
          </div>
        </div>
      )}

      {/* Actions */}
      <div className="mb-4 flex gap-2">
        <Button onClick={() => setRechargeOpen(true)}>{t('accountBalance.recharge')}</Button>
        <Button variant="secondary" onClick={() => setAdjustOpen(true)}>
          {t('accountBalance.adjust')}
        </Button>
      </div>

      {/* Transactions Table */}
      {isLoading ? (
        <div className="flex items-center justify-center py-12">
          <span className="loading loading-spinner loading-lg" />
        </div>
      ) : (
        <>
          <div className="overflow-x-auto bg-base-100 rounded-box shadow-sm">
            <table className="table table-sm">
              <thead>
                <tr className="border-b border-base-300">
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">{t('accountBalance.table.time')}</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">{t('accountBalance.table.type')}</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50 text-right">{t('accountBalance.table.amount')}</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50 text-right">
                    {t('accountBalance.table.balanceAfter')}
                  </th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">{t('accountBalance.table.description')}</th>
                </tr>
              </thead>
              <tbody>
                {transactions?.items.map((tx) => {
                  const info = TX_TYPE_LABELS[tx.type] ?? { label: tx.type, color: 'gray' as const };
                  const isCredit = tx.type === 'credit' || tx.type === 'credit_adjustment';
                  return (
                    <tr key={tx.id} className="border-b border-base-200 hover">
                      <td className="mono text-[13px]">
                        {new Date(tx.created_at).toLocaleString()}
                      </td>
                      <td>
                        <Badge variant={info.color}>{info.label}</Badge>
                      </td>
                      <td className={`mono text-right ${isCredit ? 'text-green-500' : 'text-red-500'}`}>
                        {isCredit ? '+' : '-'}${tx.amount.toFixed(4)}
                      </td>
                      <td className="mono text-right">${tx.balance_after.toFixed(4)}</td>
                      <td className="text-sm">{tx.description ?? '-'}</td>
                    </tr>
                  );
                })}
                {transactions?.items.length === 0 && (
                  <tr>
                    <td colSpan={5} className="text-center py-8 text-base-content/40">
                      {t('accountBalance.noTransactions')}
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>

          {totalPages > 1 && (
            <div className="mt-4 flex items-center justify-between text-sm">
              <span className="text-base-content/40">{t('accountBalance.pagination.total', { count: transactions?.total ?? 0 })}</span>
              <div className="join">
                <Button
                  variant="ghost"
                  size="sm"
                  disabled={page <= 1}
                  onClick={() => setPage(page - 1)}
                >
                  {t('accountBalance.pagination.previous')}
                </Button>
                <span className="px-3 flex items-center text-base-content/60">
                  {page} / {totalPages}
                </span>
                <Button
                  variant="ghost"
                  size="sm"
                  disabled={page >= totalPages}
                  onClick={() => setPage(page + 1)}
                >
                  {t('accountBalance.pagination.next')}
                </Button>
              </div>
            </div>
          )}
        </>
      )}

      {/* Recharge Modal */}
      <Modal open={rechargeOpen} onClose={() => setRechargeOpen(false)} title={t('accountBalance.rechargeModal.title')}>
        <div className="space-y-4">
          <div>
            <label className="label">
              <span className="label-text">{t('accountBalance.rechargeModal.amount')}</span>
            </label>
            <input
              type="number"
              className="input input-bordered w-full"
              value={rechargeAmount}
              onChange={(e) => setRechargeAmount(e.target.value)}
              placeholder="0.00"
              step="0.01"
              min="0"
            />
          </div>
          <div>
            <label className="label">
              <span className="label-text">{t('accountBalance.rechargeModal.descriptionLabel')}</span>
            </label>
            <input
              type="text"
              className="input input-bordered w-full"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder={t('accountBalance.rechargeModal.descriptionPlaceholder')}
            />
          </div>
          <div className="flex justify-end gap-2 pt-2">
            <Button variant="ghost" onClick={() => setRechargeOpen(false)}>
              {t('common.cancel')}
            </Button>
            <Button onClick={handleRecharge} disabled={rechargeMutation.isPending}>
              {t('accountBalance.rechargeModal.confirmRecharge')}
            </Button>
          </div>
        </div>
      </Modal>

      {/* Adjust Modal */}
      <Modal open={adjustOpen} onClose={() => setAdjustOpen(false)} title={t('accountBalance.adjustModal.title')}>
        <div className="space-y-4">
          <div>
            <label className="label">
              <span className="label-text">{t('accountBalance.adjustModal.type')}</span>
            </label>
            <select
              className="select select-bordered w-full"
              value={adjustType}
              onChange={(e) =>
                setAdjustType(e.target.value as 'credit_adjustment' | 'debit_refund')
              }
            >
              <option value="credit_adjustment">{t('accountBalance.adjustModal.creditAdjustment')}</option>
              <option value="debit_refund">{t('accountBalance.adjustModal.debitRefund')}</option>
            </select>
          </div>
          <div>
            <label className="label">
              <span className="label-text">{t('accountBalance.adjustModal.amount')}</span>
            </label>
            <input
              type="number"
              className="input input-bordered w-full"
              value={adjustAmount}
              onChange={(e) => setAdjustAmount(e.target.value)}
              placeholder="0.00"
              step="0.01"
              min="0"
            />
          </div>
          <div>
            <label className="label">
              <span className="label-text">{t('accountBalance.adjustModal.descriptionLabel')}</span>
            </label>
            <input
              type="text"
              className="input input-bordered w-full"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder={t('accountBalance.adjustModal.descriptionPlaceholder')}
            />
          </div>
          <div className="flex justify-end gap-2 pt-2">
            <Button variant="ghost" onClick={() => setAdjustOpen(false)}>
              {t('common.cancel')}
            </Button>
            <Button onClick={handleAdjust} disabled={adjustMutation.isPending}>
              {t('accountBalance.adjustModal.confirmAdjustment')}
            </Button>
          </div>
        </div>
      </Modal>
    </div>
  );
}
