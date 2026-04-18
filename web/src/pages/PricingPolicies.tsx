import { useState } from 'react';
import { DollarSign, Plus, Cpu, Globe, Pencil, Trash2 } from 'lucide-react';
import { usePricingPolicies, useCreatePricingPolicy, useUpdatePricingPolicy, useDeletePricingPolicy } from '../hooks/usePricingPolicies';
import { Button } from '../components/ui/Button';
import { Badge } from '../components/ui/Badge';
import { Modal } from '../components/ui/Modal';
import { ConfirmDialog } from '../components/ui/ConfirmDialog';
import { motion } from 'framer-motion';
import type { PricingPolicyWithCounts, UpdatePricingPolicy } from '../types';

const BILLING_TYPES: Record<string, string> = {
  per_token: 'Per Token',
  per_request: 'Per Request',
  per_character: 'Per Character',
  tiered_token: 'Tiered Token',
  hybrid: 'Hybrid',
};

// ── Config renderer for table cells ───────────────────────────────────────────
function fmt(val: unknown): string {
  if (typeof val === 'number') return `$${val.toFixed(4)}/M`;
  if (typeof val === 'object' && val !== null) return JSON.stringify(val);
  return String(val);
}

function ConfigCell({ policy }: { policy: PricingPolicyWithCounts }) {
  const cfg = policy.config ?? {};
  const bt = policy.billing_type;

  if (bt === 'per_token' || bt === 'tiered_token' || bt === 'hybrid') {
    const input = cfg['input_per_1m'] ?? cfg['input_price'];
    const output = cfg['output_per_1m'] ?? cfg['output_price'];
    const cache = cfg['cache_read_price'];
    return (
      <div className="flex flex-wrap gap-1.5">
        {(input as number | null) != null && (
          <span className="inline-flex flex-col items-center px-2 py-1 rounded bg-base-200/60 border border-base-300/30 min-w-[64px]">
            <span className="text-[9px] font-bold uppercase tracking-wider text-base-content/30">In</span>
            <span className="text-xs font-mono font-bold text-base-content">{fmt(input)}</span>
          </span>
        )}
        {(output as number | null) != null && (
          <span className="inline-flex flex-col items-center px-2 py-1 rounded bg-base-200/60 border border-base-300/30 min-w-[64px]">
            <span className="text-[9px] font-bold uppercase tracking-wider text-base-content/30">Out</span>
            <span className="text-xs font-mono font-bold text-base-content">{fmt(output)}</span>
          </span>
        )}
        {(cache as number | null) != null && (
          <span className="inline-flex flex-col items-center px-2 py-1 rounded bg-base-200/60 border border-base-300/30 min-w-[64px]">
            <span className="text-[9px] font-bold uppercase tracking-wider text-base-content/30">Cache</span>
            <span className="text-xs font-mono font-bold text-base-content">{fmt(cache)}</span>
          </span>
        )}
        {bt === 'hybrid' && (cfg['base_per_call'] ?? cfg['request_price']) != null && (
          <span className="inline-flex flex-col items-center px-2 py-1 rounded bg-base-200/60 border border-base-300/30 min-w-[64px]">
            <span className="text-[9px] font-bold uppercase tracking-wider text-base-content/30">Base</span>
            <span className="text-xs font-mono font-bold text-base-content">{fmt(cfg['base_per_call'] ?? cfg['request_price'])}</span>
          </span>
        )}
        {bt === 'tiered_token' && Array.isArray(cfg['tiers']) && (
          <span className="inline-flex items-center gap-1 text-xs text-base-content/40 italic">
            {(cfg['tiers'] as unknown[]).length} tier{((cfg['tiers'] as unknown[]).length !== 1) ? 's' : ''}
          </span>
        )}
      </div>
    );
  }

  if (bt === 'per_request') {
    const p = cfg['price_per_call'] ?? cfg['request_price'];
    return (
      <span className="inline-flex flex-col items-center px-2 py-1 rounded bg-base-200/60 border border-base-300/30">
        <span className="text-[9px] font-bold uppercase tracking-wider text-base-content/30">Per Call</span>
        <span className="text-xs font-mono font-bold text-base-content">{p != null ? fmt(p) : '—'}</span>
      </span>
    );
  }

  if (bt === 'per_character') {
    const input = cfg['input_per_1m'] ?? cfg['input_price'];
    const output = cfg['output_per_1m'] ?? cfg['output_price'];
    return (
      <div className="flex flex-wrap gap-1.5">
        {(input as number | null) != null && (
          <span className="inline-flex flex-col items-center px-2 py-1 rounded bg-base-200/60 border border-base-300/30 min-w-[64px]">
            <span className="text-[9px] font-bold uppercase tracking-wider text-base-content/30">In/Char</span>
            <span className="text-xs font-mono font-bold text-base-content">{fmt(input)}</span>
          </span>
        )}
        {(output as number | null) != null && (
          <span className="inline-flex flex-col items-center px-2 py-1 rounded bg-base-200/60 border border-base-300/30 min-w-[64px]">
            <span className="text-[9px] font-bold uppercase tracking-wider text-base-content/30">Out/Char</span>
            <span className="text-xs font-mono font-bold text-base-content">{fmt(output)}</span>
          </span>
        )}
      </div>
    );
  }

  return <span className="text-xs text-base-content/30 italic">—</span>;
}

function priceFields(
  billingType: string,
  config: Record<string, unknown>,
  setters: {
    setInputPrice: (v: string) => void;
    setOutputPrice: (v: string) => void;
    setRequestPrice: (v: string) => void;
    setCacheReadPrice: (v: string) => void;
  },
) {
  const { setInputPrice, setOutputPrice, setRequestPrice, setCacheReadPrice } = setters;
  if (billingType === 'per_token' || billingType === 'tiered_token') {
    return (
      <div className="space-y-2">
        <div className="grid grid-cols-2 gap-2">
          <div className="space-y-1.5">
            <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Input Price ($ per 1M)</label>
            <input
              type="number"
              step="0.0001"
              value={(config['input_per_1m'] as number | undefined) ?? (config['input_price'] as number | undefined) ?? ''}
              onChange={(e) => setInputPrice(e.target.value)}
              placeholder="$0.00"
              className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-primary/60 focus:ring-1 focus:ring-primary/20 transition-colors"
            />
          </div>
          <div className="space-y-1.5">
            <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Output Price ($ per 1M)</label>
            <input
              type="number"
              step="0.0001"
              value={(config['output_per_1m'] as number | undefined) ?? (config['output_price'] as number | undefined) ?? ''}
              onChange={(e) => setOutputPrice(e.target.value)}
              placeholder="$0.00"
              className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-primary/60 focus:ring-1 focus:ring-primary/20 transition-colors"
            />
          </div>
        </div>
        <div className="space-y-1.5">
          <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Cache Read Price ($ per 1M, cheaper)</label>
          <input
            type="number"
            step="0.0001"
            value={(config['cache_read_price'] as number | undefined) ?? ''}
            onChange={(e) => setCacheReadPrice(e.target.value)}
            placeholder="$0.00 (defaults to input price)"
            className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-primary/60 focus:ring-1 focus:ring-primary/20 transition-colors"
          />
        </div>
      </div>
    );
  }
  if (billingType === 'per_request' || billingType === 'hybrid') {
    return (
      <div className="space-y-1.5">
        <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Per Request Price</label>
        <input
          type="number"
          step="0.0001"
          value={(config['request_price'] as number | undefined) ?? ''}
          onChange={(e) => setRequestPrice(e.target.value)}
          placeholder="$0.00"
          className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-primary/60 focus:ring-1 focus:ring-primary/20 transition-colors"
        />
      </div>
    );
  }
  return null;
}

// ── Add Policy Modal ──────────────────────────────────────────────────────────
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
  const [cacheReadPrice, setCacheReadPrice] = useState('');

  const reset = () => {
    setName(''); setBillingType('per_token');
    setInputPrice(''); setOutputPrice('');
    setRequestPrice(''); setCacheReadPrice('');
  };
  const handleClose = () => { reset(); onClose(); };

  const buildConfig = (): Record<string, unknown> => {
    const cfg: Record<string, unknown> = {};
    if (inputPrice) cfg['input_per_1m'] = parseFloat(inputPrice);
    if (outputPrice) cfg['output_per_1m'] = parseFloat(outputPrice);
    if (requestPrice) cfg['request_price'] = parseFloat(requestPrice);
    if (cacheReadPrice) cfg['cache_read_price'] = parseFloat(cacheReadPrice);
    return cfg;
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    await onAdd({ name, billing_type: billingType, config: buildConfig() });
    handleClose();
  };

  return (
    <Modal open={open} onClose={handleClose} title="Add Pricing Policy">
      <form onSubmit={handleSubmit} className="space-y-5">
        <div className="space-y-1.5">
          <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Policy Name</label>
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            required
            placeholder="e.g. Standard GPT-4 Pricing"
            className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-primary/60 focus:ring-1 focus:ring-primary/20 transition-colors"
          />
        </div>

        <div className="space-y-1.5">
          <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Billing Type</label>
          <select
            value={billingType}
            onChange={(e) => setBillingType(e.target.value)}
            className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm text-base-content focus:outline-none focus:border-primary/60 focus:ring-1 focus:ring-primary/20 transition-colors"
          >
            {Object.entries(BILLING_TYPES).map(([val, label]) => (
              <option key={val} value={val}>{label}</option>
            ))}
          </select>
        </div>

        {priceFields(billingType, {}, { setInputPrice, setOutputPrice, setRequestPrice, setCacheReadPrice })}

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

// ── Edit Policy Modal ─────────────────────────────────────────────────────────
function EditPolicyModal({
  policy,
  open,
  onClose,
  onSave,
  isPending,
}: {
  policy: PricingPolicyWithCounts | null;
  open: boolean;
  onClose: () => void;
  onSave: (id: string, data: UpdatePricingPolicy) => Promise<void>;
  isPending: boolean;
}) {
  const [name, setName] = useState('');
  const [billingType, setBillingType] = useState('per_token');
  const [inputPrice, setInputPrice] = useState('');
  const [outputPrice, setOutputPrice] = useState('');
  const [requestPrice, setRequestPrice] = useState('');
  const [cacheReadPrice, setCacheReadPrice] = useState('');

  // Sync state when policy changes
  if (policy) {
    if (name !== policy.name) setName(policy.name);
    if (billingType !== policy.billing_type) setBillingType(policy.billing_type);
    const storedInput = (policy.config['input_per_1m'] ?? policy.config['input_price']) as number | undefined;
    if (inputPrice !== String(storedInput ?? '')) setInputPrice(String(storedInput ?? ''));
    const storedOutput = (policy.config['output_per_1m'] ?? policy.config['output_price']) as number | undefined;
    if (outputPrice !== String(storedOutput ?? '')) setOutputPrice(String(storedOutput ?? ''));
    if (requestPrice !== String(policy.config['request_price'] ?? '')) setRequestPrice(String(policy.config['request_price'] ?? ''));
    if (cacheReadPrice !== String(policy.config['cache_read_price'] ?? '')) setCacheReadPrice(String(policy.config['cache_read_price'] ?? ''));
  }

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!policy) return;
    const cfg: Record<string, unknown> = {};
    if (inputPrice) cfg['input_per_1m'] = parseFloat(inputPrice);
    if (outputPrice) cfg['output_per_1m'] = parseFloat(outputPrice);
    if (requestPrice) cfg['request_price'] = parseFloat(requestPrice);
    if (cacheReadPrice) cfg['cache_read_price'] = parseFloat(cacheReadPrice);

    await onSave(policy.id, {
      name,
      billing_type: billingType,
      config: cfg,
    });
    onClose();
  };

  return (
    <Modal open={open} onClose={onClose} title={`Edit ${policy?.name ?? 'Policy'}`}>
      <form onSubmit={handleSubmit} className="space-y-5">
        <div className="space-y-1.5">
          <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Policy Name</label>
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            required
            className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-primary/60 focus:ring-1 focus:ring-primary/20 transition-colors"
          />
        </div>

        <div className="space-y-1.5">
          <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Billing Type</label>
          <select
            value={billingType}
            onChange={(e) => setBillingType(e.target.value)}
            className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm text-base-content focus:outline-none focus:border-primary/60 focus:ring-1 focus:ring-primary/20 transition-colors"
          >
            {Object.entries(BILLING_TYPES).map(([val, label]) => (
              <option key={val} value={val}>{label}</option>
            ))}
          </select>
        </div>

        {priceFields(billingType, policy?.config ?? {}, { setInputPrice, setOutputPrice, setRequestPrice, setCacheReadPrice })}

        <div className="flex gap-2 pt-1">
          <Button type="submit" variant="primary" loading={isPending} className="flex-1">
            Save Changes
          </Button>
          <Button type="button" variant="ghost" onClick={onClose}>
            Cancel
          </Button>
        </div>
      </form>
    </Modal>
  );
}

// ── Main Page ─────────────────────────────────────────────────────────────────
export default function PricingPolicies() {
  const { data: policies, isLoading } = usePricingPolicies();
  const createMutation = useCreatePricingPolicy();
  const updateMutation = useUpdatePricingPolicy();
  const deleteMutation = useDeletePricingPolicy();
  const [isAdding, setIsAdding] = useState(false);
  const [editingPolicy, setEditingPolicy] = useState<PricingPolicyWithCounts | null>(null);

  const total = policies?.length ?? 0;

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-24">
        <div className="flex flex-col items-center gap-3">
          <div className="w-8 h-8 rounded-full border-2 border-primary/30 border-t-primary animate-spin" />
          <span className="text-xs text-base-content/35 font-medium">Loading pricing policies…</span>
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
            <h1 className="text-2xl font-bold tracking-tight text-base-content">Pricing Policies</h1>
            {total > 0 && (
              <span className="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-bold uppercase tracking-widest bg-base-200/70 text-base-content/40 border border-base-300/50">
                {total}
              </span>
            )}
          </div>
          <p className="text-sm text-base-content/35">
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
          <h3 className="text-lg font-semibold text-base-content/60 mb-1.5">No pricing policies yet</h3>
          <p className="text-sm text-base-content/30 mb-6 text-center max-w-xs">
            Create pricing policies to apply billing rules to models and channels.
          </p>
          <Button variant="primary" size="sm" onClick={() => setIsAdding(true)}>
            Create First Policy
          </Button>
        </motion.div>
      ) : (
        <div className="bg-base-100 rounded-xl border border-base-300 overflow-hidden">
          <table className="table">
            <thead>
              <tr className="border-b border-base-300">
                <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Name</th>
                <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Billing</th>
                <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Config</th>
                <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Models</th>
                <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Channel Models</th>
                <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Created</th>
                <th className="w-20"></th>
              </tr>
            </thead>
            <tbody>
              {policies!.map((policy) => (
                <tr key={policy.id} className="border-b border-base-200 hover:bg-base-50">
                  <td className="font-mono font-medium">{policy.name}</td>
                  <td>
                    <Badge variant="blue">{BILLING_TYPES[policy.billing_type] ?? policy.billing_type}</Badge>
                  </td>
                  <td><ConfigCell policy={policy} /></td>
                  <td className="text-base-content/60">
                    <span className="inline-flex items-center gap-1.5">
                      <Cpu className="h-3.5 w-3.5 text-base-content/30" />
                      {policy.model_count}
                    </span>
                  </td>
                  <td className="text-base-content/60">
                    <span className="inline-flex items-center gap-1.5">
                      <Globe className="h-3.5 w-3.5 text-base-content/30" />
                      {policy.channel_model_count}
                    </span>
                  </td>
                  <td className="text-base-content/40 text-sm">
                    {new Date(policy.created_at).toLocaleDateString()}
                  </td>
                  <td>
                    <div className="flex items-center gap-1">
                      <button
                        onClick={() => setEditingPolicy(policy)}
                        className="btn btn-ghost btn-sm btn-circle"
                        title="Edit"
                      >
                        <Pencil className="h-3.5 w-3.5" />
                      </button>
                      <ConfirmDialog
                        title={`Delete "${policy.name}"?`}
                        onConfirm={() => deleteMutation.mutate(policy.id)}
                        okText="Delete"
                      >
                        <button
                          className="btn btn-ghost btn-sm btn-circle text-error"
                          title="Delete"
                          disabled={deleteMutation.isPending}
                        >
                          <Trash2 className="h-3.5 w-3.5" />
                        </button>
                      </ConfirmDialog>
                    </div>
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

      <EditPolicyModal
        policy={editingPolicy}
        open={editingPolicy !== null}
        onClose={() => setEditingPolicy(null)}
        onSave={async (id, data) => {
          await updateMutation.mutateAsync({ id, input: data });
        }}
        isPending={updateMutation.isPending}
      />
    </div>
  );
}
