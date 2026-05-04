import { useState } from 'react';
import { useParams } from 'react-router-dom';
import { DollarSign, Eye } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useUserBalance, useRechargeUser, useAdjustUser, useRequestDetails } from '../hooks/useAccounts';
import { Button } from '../components/ui/Button';
import { Badge } from '../components/ui/Badge';
import { Modal } from '../components/ui/Modal';
import { Drawer } from '../components/ui/Drawer';

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
  const [selectedRequestId, setSelectedRequestId] = useState<string | null>(null);

  const { data, isLoading } = useUserBalance(userId ?? '', page, pageSize);
  const rechargeMutation = useRechargeUser();
  const adjustMutation = useAdjustUser();
  const { data: requestDetails, isLoading: detailsLoading } = useRequestDetails(selectedRequestId);

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

  function DetailRow({ label, value }: { label: string; value: string }) {
    return (
      <div className="flex items-center justify-between text-sm">
        <span className="text-base-content/60">{label}</span>
        <span className="font-medium text-base-content">{value}</span>
      </div>
    );
  }

  function formatTokens(val: number | null): string {
    if (val === null) return '-';
    return val.toLocaleString();
  }

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
                  const canDrillDown = !!tx.request_id;
                  return (
                    <tr
                      key={tx.id}
                      className={`border-b border-base-200 hover ${canDrillDown ? 'cursor-pointer' : ''}`}
                      onClick={() => canDrillDown && setSelectedRequestId(tx.request_id)}
                    >
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
                      <td className="text-sm">
                        <div className="flex items-center gap-1">
                          <span>{tx.description ?? '-'}</span>
                          {canDrillDown && (
                            <Eye className="h-3.5 w-3.5 text-base-content/30 shrink-0" />
                          )}
                        </div>
                      </td>
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

      {/* Request Details Drawer */}
      <Drawer
        open={!!selectedRequestId}
        onClose={() => setSelectedRequestId(null)}
        title={t('accountBalance.requestDetails.title')}
        width={520}
      >
        {detailsLoading ? (
          <div className="flex items-center justify-center py-12">
            <span className="loading loading-spinner loading-md" />
          </div>
        ) : requestDetails ? (
          <div className="space-y-6">
            {/* Transaction Info */}
            {requestDetails.transaction && (
              <section>
                <h4 className="text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-3">
                  {t('accountBalance.requestDetails.transactionInfo')}
                </h4>
                <div className="space-y-2">
                  <DetailRow label={t('accountBalance.table.amount')} value={`$${requestDetails.transaction.amount.toFixed(4)}`} />
                  <DetailRow label={t('accountBalance.table.balanceAfter')} value={`$${requestDetails.transaction.balance_after.toFixed(4)}`} />
                  <DetailRow label={t('accountBalance.table.description')} value={requestDetails.transaction.description ?? '-'} />
                </div>
              </section>
            )}

            {/* Request ID */}
            <section>
              <h4 className="text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-3">
                {t('accountBalance.requestDetails.requestId')}
              </h4>
              <div className="bg-base-200 rounded-md px-3 py-2 font-mono text-xs break-all select-all">
                {selectedRequestId}
              </div>
            </section>

            {/* Usage Record */}
            {requestDetails.usage ? (
              <section>
                <h4 className="text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-3">
                  {t('accountBalance.requestDetails.usageInfo')}
                </h4>
                <div className="space-y-2">
                  <DetailRow label={t('accountBalance.requestDetails.model')} value={requestDetails.usage.model_name} />
                  <DetailRow label={t('accountBalance.requestDetails.inputTokens')} value={formatTokens(requestDetails.usage.input_tokens)} />
                  <DetailRow label={t('accountBalance.requestDetails.outputTokens')} value={formatTokens(requestDetails.usage.output_tokens)} />
                  <DetailRow label={t('accountBalance.requestDetails.cacheReadTokens')} value={formatTokens(requestDetails.usage.cache_read_tokens)} />
                  <DetailRow label={t('accountBalance.requestDetails.cacheCreationTokens')} value={formatTokens(requestDetails.usage.cache_creation_tokens)} />
                  <DetailRow label={t('accountBalance.requestDetails.cost')} value={`$${requestDetails.usage.cost.toFixed(4)}`} />
                </div>
              </section>
            ) : (
              <section>
                <h4 className="text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-3">
                  {t('accountBalance.requestDetails.usageInfo')}
                </h4>
                <p className="text-sm text-base-content/40">{t('accountBalance.requestDetails.noData')}</p>
              </section>
            )}

            {/* Audit Info */}
            {requestDetails.audit ? (
              <section>
                <h4 className="text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-3">
                  {t('accountBalance.requestDetails.auditInfo')}
                </h4>
                <div className="space-y-2">
                  <DetailRow label={t('accountBalance.requestDetails.status')} value={`${requestDetails.audit.status_code}`} />
                  <DetailRow label={t('accountBalance.requestDetails.latency')} value={`${requestDetails.audit.latency_ms}ms`} />
                  <DetailRow label={t('accountBalance.requestDetails.protocol')} value={requestDetails.audit.protocol} />
                  <DetailRow label={t('accountBalance.requestDetails.stream')} value={requestDetails.audit.stream ? t('common.yes') : t('common.no')} />
                  {requestDetails.audit.channel_name && (
                    <DetailRow label={t('accountBalance.requestDetails.channel')} value={requestDetails.audit.channel_name} />
                  )}
                </div>
              </section>
            ) : null}
          </div>
        ) : (
          <div className="flex items-center justify-center py-12">
            <p className="text-sm text-base-content/40">{t('accountBalance.requestDetails.noData')}</p>
          </div>
        )}
      </Drawer>
    </div>
  );
}
