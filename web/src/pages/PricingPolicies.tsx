import { useState } from 'react';
import { DollarSign, Plus, Cpu, Globe } from 'lucide-react';
import { usePricingPolicies, useCreatePricingPolicy } from '../hooks/usePricingPolicies';
import { Button } from '../components/ui/Button';
import { Badge } from '../components/ui/Badge';
import { Modal } from '../components/ui/Modal';
import { motion } from 'framer-motion';

const BILLING_TYPES: Record<string, string> = {
  per_token: 'Per Token',
  per_request: 'Per Request',
  per_character: 'Per Character',
  tiered_token: 'Tiered Token',
  hybrid: 'Hybrid',
};

function AddPolicyModal({
  open,
  onClose,
  onAdd,
  isPending,
}: {
  open: boolean;
  onClose: () => void;
  onAdd: (data: { name: string; billing_type: string; config: Record<string, unknown> }) => Promise<void>;
  isPending: boolean;
}) {
  const [name, setName] = useState('');
  const [billingType, setBillingType] = useState('per_token');
  const [inputPrice, setInputPrice] = useState('');
  const [outputPrice, setOutputPrice] = useState('');
  const [requestPrice, setRequestPrice] = useState('');

  const reset = () => {
    setName('');
    setBillingType('per_token');
    setInputPrice('');
    setOutputPrice('');
    setRequestPrice('');
  };

  const handleClose = () => { reset(); onClose(); };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const config: Record<string, unknown> = {};
    if (inputPrice) config.input_price = parseFloat(inputPrice);
    if (outputPrice) config.output_price = parseFloat(outputPrice);
    if (requestPrice) config.request_price = parseFloat(requestPrice);

    await onAdd({ name, billing_type: billingType, config });
    handleClose();
  };

  return (
    <Modal open={open} onClose={handleClose} title="Add Pricing Policy">
      <form onSubmit={handleSubmit} className="space-y-5">
        <div className="space-y-1.5">
          <label className="text-[11px] font-semibold uppercase tracking-wider text-base-content/50">Policy Name</label>
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            required
            placeholder="e.g. Standard GPT-4 Pricing"
            className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-[13px] font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-primary/60 focus:ring-1 focus:ring-primary/20 transition-colors"
          />
        </div>

        <div className="space-y-1.5">
          <label className="text-[11px] font-semibold uppercase tracking-wider text-base-content/50">Billing Type</label>
          <select
            value={billingType}
            onChange={(e) => setBillingType(e.target.value)}
            className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-[13px] text-base-content focus:outline-none focus:border-primary/60 focus:ring-1 focus:ring-primary/20 transition-colors"
          >
            {Object.entries(BILLING_TYPES).map(([val, label]) => (
              <option key={val} value={val}>{label}</option>
            ))}
          </select>
        </div>

        {(billingType === 'per_token' || billingType === 'tiered_token') && (
          <div className="grid grid-cols-2 gap-2">
            <div className="space-y-1.5">
              <label className="text-[11px] font-semibold uppercase tracking-wider text-base-content/50">Input Price (per 1M)</label>
              <input
                type="number"
                step="0.0001"
                value={inputPrice}
                onChange={(e) => setInputPrice(e.target.value)}
                placeholder="$0.00"
                className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-[13px] font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-primary/60 focus:ring-1 focus:ring-primary/20 transition-colors"
              />
            </div>
            <div className="space-y-1.5">
              <label className="text-[11px] font-semibold uppercase tracking-wider text-base-content/50">Output Price (per 1M)</label>
              <input
                type="number"
                step="0.0001"
                value={outputPrice}
                onChange={(e) => setOutputPrice(e.target.value)}
                placeholder="$0.00"
                className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-[13px] font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-primary/60 focus:ring-1 focus:ring-primary/20 transition-colors"
              />
            </div>
          </div>
        )}

        {(billingType === 'per_request' || billingType === 'hybrid') && (
          <div className="space-y-1.5">
            <label className="text-[11px] font-semibold uppercase tracking-wider text-base-content/50">Per Request Price</label>
            <input
              type="number"
              step="0.0001"
              value={requestPrice}
              onChange={(e) => setRequestPrice(e.target.value)}
              placeholder="$0.00"
              className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-[13px] font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-primary/60 focus:ring-1 focus:ring-primary/20 transition-colors"
            />
          </div>
        )}

        <div className="flex gap-2 pt-1">
          <Button type="submit" variant="primary" loading={isPending} className="flex-1">
            Create Policy
          </Button>
          <Button type="button" variant="ghost" onClick={handleClose}>
            Cancel
          </Button>
        </div>
      </form>
    </Modal>
  );
}

export default function PricingPolicies() {
  const { data: policies, isLoading } = usePricingPolicies();
  const createMutation = useCreatePricingPolicy();
  const [isAdding, setIsAdding] = useState(false);

  const total = policies?.length ?? 0;

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-24">
        <div className="flex flex-col items-center gap-3">
          <div className="w-8 h-8 rounded-full border-2 border-primary/30 border-t-primary animate-spin" />
          <span className="text-[12px] text-base-content/35 font-medium">Loading pricing policies…</span>
        </div>
      </div>
    );
  }

  return (
    <div className="px-6 pb-8">
      <motion.div
        initial={{ opacity: 0, y: -8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.4 }}
        className="mb-6 flex items-start justify-between pt-8"
      >
        <div>
          <div className="flex items-center gap-3 mb-1">
            <h1 className="text-[22px] font-bold tracking-tight text-base-content">Pricing Policies</h1>
            {total > 0 && (
              <span className="inline-flex items-center px-2 py-0.5 rounded-full text-[10px] font-bold uppercase tracking-widest bg-base-200/70 text-base-content/40 border border-base-300/50">
                {total}
              </span>
            )}
          </div>
          <p className="text-[13px] text-base-content/35">
            {total === 0 ? 'Configure pricing rules for models and channels' : `${total} pricing policies configured`}
          </p>
        </div>

        <motion.div
          initial={{ opacity: 0, scale: 0.92 }}
          animate={{ opacity: 1, scale: 1 }}
          transition={{ duration: 0.2 }}
        >
          <Button icon={<Plus className="h-4 w-4" />} onClick={() => setIsAdding(true)} size="sm">
            Add Policy
          </Button>
        </motion.div>
      </motion.div>

      {total === 0 ? (
        <motion.div
          initial={{ opacity: 0, y: 24 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5 }}
          className="flex flex-col items-center justify-center py-24 px-4"
        >
          <div className="w-16 h-16 rounded-2xl bg-base-200/60 flex items-center justify-center mb-6">
            <DollarSign className="h-8 w-8 text-base-content/20" />
          </div>
          <h3 className="text-[15px] font-semibold text-base-content/60 mb-1.5">No pricing policies yet</h3>
          <p className="text-[13px] text-base-content/30 mb-6 text-center max-w-xs">
            Create pricing policies to apply billing rules to models and channels.
          </p>
          <Button variant="primary" size="sm" onClick={() => setIsAdding(true)}>
            Create First Policy
          </Button>
        </motion.div>
      ) : (
        <div className="overflow-x-auto bg-base-100 rounded-box border border-base-300 shadow-sm">
          <table className="table table-sm">
            <thead>
              <tr className="border-b border-base-300">
                <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Name</th>
                <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Billing</th>
                <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Models</th>
                <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Channel Models</th>
                <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Created</th>
              </tr>
            </thead>
            <tbody>
              {policies!.map((policy) => (
                <tr key={policy.id} className="border-b border-base-200 hover">
                  <td className="font-mono font-medium">{policy.name}</td>
                  <td>
                    <Badge variant="blue">{BILLING_TYPES[policy.billing_type] ?? policy.billing_type}</Badge>
                  </td>
                  <td className="mono text-base-content/60">
                    <span className="inline-flex items-center gap-1">
                      <Cpu className="h-3 w-3 text-base-content/30" />
                      {policy.model_count}
                    </span>
                  </td>
                  <td className="mono text-base-content/60">
                    <span className="inline-flex items-center gap-1">
                      <Globe className="h-3 w-3 text-base-content/30" />
                      {policy.channel_model_count}
                    </span>
                  </td>
                  <td className="mono text-base-content/40 text-[12px]">
                    {new Date(policy.created_at).toLocaleDateString()}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      <AddPolicyModal
        open={isAdding}
        onClose={() => setIsAdding(false)}
        onAdd={async (data) => {
          await createMutation.mutateAsync(data);
        }}
        isPending={createMutation.isPending}
      />
    </div>
  );
}
