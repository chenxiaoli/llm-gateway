import { useState, useEffect } from 'react';
import { useAllModels, useCreateGlobalModel, useUpdateGlobalModel } from '../hooks/useModels';
import { useProviders } from '../hooks/useProviders';
import { Link } from 'react-router-dom';
import { Button } from '../components/ui/Button';
import { Modal } from '../components/ui/Modal';
import { Badge } from '../components/ui/Badge';
import { Globe, Plus, Cpu, Activity, Pencil, Sparkles } from 'lucide-react';
import type { CreateGlobalModelRequest, ModelWithProvider } from '../types';
import { motion, AnimatePresence } from 'framer-motion';

// Billing types
const BILLING_TYPES = [
  { value: 'per_token', label: 'Per Token', desc: 'Per-token pricing for input & output' },
  { value: 'per_request', label: 'Per Request', desc: 'Flat fee per API call' },
  { value: 'per_character', label: 'Per Character', desc: 'Pricing per character' },
  { value: 'tiered_token', label: 'Tiered Token', desc: 'Volume-based token pricing' },
  { value: 'hybrid', label: 'Hybrid', desc: 'Combined token + request' },
] as const;

export type BillingType = typeof BILLING_TYPES[number]['value'];

// ── Model Card ───────────────────────────────────────────────────────────────
interface ModelCardProps {
  model: ModelWithProvider;
  providerName: string;
  index: number;
  onEdit: (model: ModelWithProvider) => void;
}

function ModelCard({ model, providerName, index, onEdit }: ModelCardProps) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.45, delay: index * 0.05, ease: [0.25, 0.1, 0.25, 1] }}
      whileHover={{ y: -3 }}
      className="group relative"
    >
      <div className="relative rounded-xl border border-base-300/60 bg-base-100/70 backdrop-blur-sm overflow-hidden transition-all duration-200 group-hover:border-accent/30 group-hover:bg-base-100/90">
        {/* Accent line top */}
        <div className={`absolute top-0 left-0 right-0 h-[2px] transition-opacity duration-200 ${model.enabled ? 'bg-gradient-to-r from-accent to-accent/40 opacity-100' : 'opacity-0'}`} />

        <div className="p-5">
          {/* Header */}
          <div className="flex items-start justify-between mb-4">
            <div className="flex items-center gap-2.5">
              <div className="w-8 h-8 rounded-lg bg-base-200/70 flex items-center justify-center shrink-0">
                <Cpu className="h-4 w-4 text-accent" />
              </div>
              <div>
                <div className="font-mono text-[13px] font-semibold text-base-content leading-tight">{model.name}</div>
                <div className="flex items-center gap-1.5 mt-0.5">
                  <Globe className="h-3 w-3 text-base-content/35" />
                  <span className="text-[11px] text-base-content/45 font-medium">{providerName}</span>
                </div>
              </div>
            </div>

            {/* Status */}
            <div className={`flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[10px] font-semibold tracking-wide uppercase transition-colors ${
              model.enabled
                ? 'bg-accent/10 text-accent'
                : 'bg-base-300/40 text-base-content/35'
            }`}>
              <span className={`w-1.5 h-1.5 rounded-full ${model.enabled ? 'bg-accent animate-pulse' : 'bg-base-content/25'}`} />
              {model.enabled ? 'Active' : 'Disabled'}
            </div>
          </div>

          {/* Billing & Pricing */}
          <div className="grid grid-cols-3 gap-2 mb-4">
            <div className="rounded-lg bg-base-200/40 px-3 py-2">
              <div className="text-[10px] uppercase tracking-widest text-base-content/30 font-semibold mb-1">Billing</div>
              <Badge variant="blue">{model.billing_type}</Badge>
            </div>
            <div className="rounded-lg bg-base-200/40 px-3 py-2">
              <div className="text-[10px] uppercase tracking-widest text-base-content/30 font-semibold mb-1">Input</div>
              <div className="text-[12px] font-mono font-semibold text-base-content/80">
                {model.input_price != null ? `$${model.input_price.toFixed(4)}` : '—'}
              </div>
            </div>
            <div className="rounded-lg bg-base-200/40 px-3 py-2">
              <div className="text-[10px] uppercase tracking-widest text-base-content/30 font-semibold mb-1">Output</div>
              <div className="text-[12px] font-mono font-semibold text-base-content/80">
                {model.output_price != null ? `$${model.output_price.toFixed(4)}` : '—'}
              </div>
            </div>
          </div>

          {/* Request price (if applicable) */}
          {(model.billing_type === 'per_request' || model.billing_type === 'hybrid') && model.request_price != null && (
            <div className="rounded-lg bg-base-200/40 px-3 py-2 mb-4">
              <div className="flex items-center justify-between">
                <span className="text-[10px] uppercase tracking-widest text-base-content/30 font-semibold">Per Request</span>
                <span className="text-[12px] font-mono font-semibold text-base-content/80">${model.request_price.toFixed(4)}</span>
              </div>
            </div>
          )}

          {/* Pricing Policy */}
          <div className="flex items-center gap-2 px-3 py-2 rounded-lg bg-base-200/30 border border-base-300/20 mb-4">
            <div className="text-[10px] uppercase tracking-widest text-base-content/30 font-semibold shrink-0">Pricing Policy</div>
            {model.pricing_policy_id ? (
              <div className="flex items-center gap-1.5">
                <span className="inline-flex items-center px-1.5 py-0.5 rounded bg-accent/8 text-accent/80 text-[10px] font-mono font-semibold border border-accent/15">
                  {model.pricing_policy_id}
                </span>
              </div>
            ) : (
              <span className="text-[11px] text-base-content/25 italic">No policy assigned</span>
            )}
          </div>

          {/* Footer */}
          <div className="flex items-center justify-between pt-3 border-t border-base-300/40">
            <button
              onClick={() => onEdit(model)}
              className="flex items-center gap-1.5 text-[11px] font-medium text-base-content/40 hover:text-accent transition-colors duration-150 cursor-pointer"
            >
              <Pencil className="h-3 w-3" />
              Edit
            </button>
            <div className="flex items-center gap-1">
              {model.enabled && (
                <div className="flex items-center gap-1 text-[10px] text-base-content/30">
                  <Activity className="h-3 w-3" />
                  <span>Ready</span>
                </div>
              )}
            </div>
          </div>
        </div>
      </div>
    </motion.div>
  );
}

// ── Empty State ───────────────────────────────────────────────────────────────
function EmptyState({ hasProviders, onAddClick }: { hasProviders: boolean; onAddClick: () => void }) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 24 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.5 }}
      className="flex flex-col items-center justify-center py-24 px-4"
    >
      <div className="relative mb-6">
        <div className="w-16 h-16 rounded-2xl bg-base-200/60 flex items-center justify-center">
          <Cpu className="h-8 w-8 text-base-content/20" />
        </div>
        <div className="absolute -bottom-1 -right-1 w-6 h-6 rounded-lg bg-accent/10 border border-accent/20 flex items-center justify-center">
          <Plus className="h-3 w-3 text-accent" />
        </div>
      </div>
      <h3 className="text-[15px] font-semibold text-base-content/60 mb-1.5">No models yet</h3>
      <p className="text-[13px] text-base-content/30 mb-6 text-center max-w-xs">
        {hasProviders
          ? 'Add your first AI model to start routing requests through the gateway.'
          : 'You need to add a provider before you can configure models.'}
      </p>
      {hasProviders ? (
        <Button variant="primary" size="sm" onClick={onAddClick}>
          Add First Model
        </Button>
      ) : (
        <Link to="/console/channels">
          <Button variant="secondary" size="sm">Add Provider First</Button>
        </Link>
      )}
    </motion.div>
  );
}

// ── Add Model Modal ──────────────────────────────────────────────────────────

function AddModelModal({
  open,
  onClose,
  providers,
  onAdd,
  isPending,
}: {
  open: boolean;
  onClose: () => void;
  providers?: Array<{ id: string; name: string }>;
  onAdd: (data: CreateGlobalModelRequest) => Promise<void>;
  isPending: boolean;
}) {
  const [providerId, setProviderId] = useState('');
  const [name, setName] = useState('');
  const [billingType, setBillingType] = useState<string>('per_token');
  const [inputPrice, setInputPrice] = useState('');
  const [outputPrice, setOutputPrice] = useState('');
  const [requestPrice, setRequestPrice] = useState('');

  const reset = () => {
    setProviderId(''); setName(''); setBillingType('per_token');
    setInputPrice(''); setOutputPrice(''); setRequestPrice('');
  };

  const handleClose = () => { reset(); onClose(); };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    await onAdd({
      provider_id: providerId,
      name,
      billing_type: billingType,
      input_price: inputPrice ? parseFloat(inputPrice) : undefined,
      output_price: outputPrice ? parseFloat(outputPrice) : undefined,
      request_price: requestPrice ? parseFloat(requestPrice) : undefined,
    });
    handleClose();
  };

  return (
    <Modal open={open} onClose={handleClose} title="Add Model">
      <form onSubmit={handleSubmit} className="space-y-5">
        {/* Provider */}
        <div className="space-y-1.5">
          <label className="text-[11px] font-semibold uppercase tracking-wider text-base-content/50">Provider</label>
          <select
            value={providerId}
            onChange={(e) => setProviderId(e.target.value)}
            required
            className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-[13px] text-base-content focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
          >
            <option value="">Select a provider...</option>
            {providers?.map((p) => (
              <option key={p.id} value={p.id}>{p.name}</option>
            ))}
          </select>
        </div>

        {/* Model name */}
        <div className="space-y-1.5">
          <label className="text-[11px] font-semibold uppercase tracking-wider text-base-content/50">Model Name</label>
          <div className="relative">
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              required
              placeholder="e.g. gpt-4o, claude-3-5-sonnet"
              className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 pl-9 pr-3 text-[13px] font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
            />
            <Sparkles className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-base-content/25" />
          </div>
        </div>

        {/* Billing type */}
        <div className="space-y-1.5">
          <label className="text-[11px] font-semibold uppercase tracking-wider text-base-content/50">Billing Type</label>
          <select
            value={billingType}
            onChange={(e) => setBillingType(e.target.value)}
            required
            className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-[13px] text-base-content focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
          >
            {BILLING_TYPES.map((opt) => (
              <option key={opt.value} value={opt.value}>{opt.label} - {opt.desc}</option>
            ))}
          </select>
        </div>

        {/* Pricing */}
        <div className="space-y-1.5">
          <label className="text-[11px] font-semibold uppercase tracking-wider text-base-content/50">
            {billingType === 'per_request' || billingType === 'hybrid' ? 'Request Pricing' : 'Token Pricing (per 1M tokens)'}
          </label>
          {(billingType === 'per_token' || billingType === 'tiered_token') && (
            <div className="grid grid-cols-2 gap-2">
              <div className="relative">
                <span className="absolute left-3 top-1/2 -translate-y-1/2 text-[11px] text-base-content/35 font-mono">$</span>
                <input
                  type="number"
                  step="0.0001"
                  value={inputPrice}
                  onChange={(e) => setInputPrice(e.target.value)}
                  placeholder="Input price"
                  className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 pl-7 pr-3 text-[13px] font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
                />
                <span className="absolute right-3 top-1/2 -translate-y-1/2 text-[10px] text-base-content/25">In</span>
              </div>
              <div className="relative">
                <span className="absolute left-3 top-1/2 -translate-y-1/2 text-[11px] text-base-content/35 font-mono">$</span>
                <input
                  type="number"
                  step="0.0001"
                  value={outputPrice}
                  onChange={(e) => setOutputPrice(e.target.value)}
                  placeholder="Output price"
                  className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 pl-7 pr-3 text-[13px] font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
                />
                <span className="absolute right-3 top-1/2 -translate-y-1/2 text-[10px] text-base-content/25">Out</span>
              </div>
            </div>
          )}
          {(billingType === 'per_request' || billingType === 'hybrid') && (
            <div className="relative">
              <span className="absolute left-3 top-1/2 -translate-y-1/2 text-[11px] text-base-content/35 font-mono">$</span>
              <input
                type="number"
                step="0.0001"
                value={requestPrice}
                onChange={(e) => setRequestPrice(e.target.value)}
                placeholder="0.00"
                className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 pl-7 pr-3 text-[13px] font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
              />
              <span className="absolute right-3 top-1/2 -translate-y-1/2 text-[10px] text-base-content/25">/req</span>
            </div>
          )}
          {billingType === 'per_character' && (
            <div className="relative">
              <span className="absolute left-3 top-1/2 -translate-y-1/2 text-[11px] text-base-content/35 font-mono">$</span>
              <input
                type="number"
                step="0.0001"
                value={inputPrice}
                onChange={(e) => setInputPrice(e.target.value)}
                placeholder="Price per character"
                className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 pl-7 pr-3 text-[13px] font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
              />
            </div>
          )}
          <p className="text-[10px] text-base-content/25">
            Leave blank to inherit from provider pricing
          </p>
        </div>

        {/* Actions */}
        <div className="flex items-center gap-2 pt-1">
          <Button type="submit" variant="primary" loading={isPending} className="flex-1">
            Add Model
          </Button>
          <Button type="button" variant="ghost" onClick={handleClose}>
            Cancel
          </Button>
        </div>
      </form>
    </Modal>
  );
}

// ── Edit Model Modal ─────────────────────────────────────────────────────────

function EditModelModal({
  model,
  open,
  onClose,
  onSave,
  isPending,
}: {
  model: ModelWithProvider | null;
  open: boolean;
  onClose: () => void;
  onSave: (data: { billing_type: string; input_price?: number; output_price?: number; request_price?: number; enabled: boolean }) => Promise<void>;
  isPending: boolean;
}) {
  const [billingType, setBillingType] = useState<string>('per_token');
  const [inputPrice, setInputPrice] = useState('');
  const [outputPrice, setOutputPrice] = useState('');
  const [requestPrice, setRequestPrice] = useState('');
  const [enabled, setEnabled] = useState(false);

  // Sync state when model changes
  useEffect(() => {
    if (model) {
      setBillingType(model.billing_type || 'per_token');
      setInputPrice(model.input_price != null ? String(model.input_price) : '');
      setOutputPrice(model.output_price != null ? String(model.output_price) : '');
      setRequestPrice(model.request_price != null ? String(model.request_price) : '');
      setEnabled(model.enabled);
    }
  }, [model]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    await onSave({
      billing_type: billingType,
      input_price: inputPrice ? parseFloat(inputPrice) : undefined,
      output_price: outputPrice ? parseFloat(outputPrice) : undefined,
      request_price: requestPrice ? parseFloat(requestPrice) : undefined,
      enabled,
    });
    onClose();
  };

  return (
    <Modal open={open} onClose={onClose} title={`Edit ${model?.name ?? 'Model'}`}>
      <form onSubmit={handleSubmit} className="space-y-5">
        {/* Status toggle */}
        <div className="flex items-center justify-between rounded-lg border border-base-300/50 bg-base-200/30 px-4 py-3">
          <div>
            <div className="text-[12px] font-semibold text-base-content/80">Enabled</div>
            <div className="text-[11px] text-base-content/35">Allow this model to receive traffic</div>
          </div>
          <button
            type="button"
            onClick={() => setEnabled(v => !v)}
            className={`relative inline-flex h-5 w-9 items-center rounded-full transition-colors duration-200 cursor-pointer ${enabled ? 'bg-accent' : 'bg-base-300'}`}
          >
            <span className={`inline-block h-3 w-3 rounded-full bg-white transition-transform duration-200 ${enabled ? 'translate-x-5' : 'translate-x-1'}`} />
          </button>
        </div>

        {/* Billing type */}
        <div className="space-y-1.5">
          <label className="text-[11px] font-semibold uppercase tracking-wider text-base-content/50">Billing Type</label>
          <select
            value={billingType}
            onChange={(e) => setBillingType(e.target.value)}
            required
            className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-[13px] text-base-content focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
          >
            {BILLING_TYPES.map((opt) => (
              <option key={opt.value} value={opt.value}>{opt.label} - {opt.desc}</option>
            ))}
          </select>
        </div>

        {/* Pricing */}
        <div className="space-y-1.5">
          <label className="text-[11px] font-semibold uppercase tracking-wider text-base-content/50">
            {billingType === 'per_request' || billingType === 'hybrid' ? 'Request Pricing' : 'Token Pricing (per 1M tokens)'}
          </label>
          {(billingType === 'per_token' || billingType === 'tiered_token') && (
            <div className="grid grid-cols-2 gap-2">
              <div className="relative">
                <span className="absolute left-3 top-1/2 -translate-y-1/2 text-[11px] text-base-content/35 font-mono">$</span>
                <input
                  type="number"
                  step="0.0001"
                  value={inputPrice}
                  onChange={(e) => setInputPrice(e.target.value)}
                  placeholder="Input price"
                  className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 pl-7 pr-3 text-[13px] font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
                />
                <span className="absolute right-3 top-1/2 -translate-y-1/2 text-[10px] text-base-content/25">In</span>
              </div>
              <div className="relative">
                <span className="absolute left-3 top-1/2 -translate-y-1/2 text-[11px] text-base-content/35 font-mono">$</span>
                <input
                  type="number"
                  step="0.0001"
                  value={outputPrice}
                  onChange={(e) => setOutputPrice(e.target.value)}
                  placeholder="Output price"
                  className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 pl-7 pr-3 text-[13px] font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
                />
                <span className="absolute right-3 top-1/2 -translate-y-1/2 text-[10px] text-base-content/25">Out</span>
              </div>
            </div>
          )}
          {(billingType === 'per_request' || billingType === 'hybrid') && (
            <div className="relative">
              <span className="absolute left-3 top-1/2 -translate-y-1/2 text-[11px] text-base-content/35 font-mono">$</span>
              <input
                type="number"
                step="0.0001"
                value={requestPrice}
                onChange={(e) => setRequestPrice(e.target.value)}
                placeholder="0.00"
                className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 pl-7 pr-3 text-[13px] font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
              />
              <span className="absolute right-3 top-1/2 -translate-y-1/2 text-[10px] text-base-content/25">/req</span>
            </div>
          )}
        </div>

        {/* Actions */}
        <div className="flex items-center gap-2 pt-1">
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
export default function Models() {
  const { data: models, isLoading } = useAllModels();
  const { data: providers } = useProviders();
  const createMutation = useCreateGlobalModel();
  const updateMutation = useUpdateGlobalModel();
  const [isAdding, setIsAdding] = useState(false);
  const [editingModel, setEditingModel] = useState<ModelWithProvider | null>(null);

  // Stats
  const totalModels = models?.length ?? 0;
  const activeModels = models?.filter(m => m.enabled).length ?? 0;

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-24">
        <div className="flex flex-col items-center gap-3">
          <div className="w-8 h-8 rounded-full border-2 border-accent/30 border-t-accent animate-spin" />
          <span className="text-[12px] text-base-content/35 font-medium">Loading models...</span>
        </div>
      </div>
    );
  }

  return (
    <div className="px-6 pb-8">
      {/* Page header */}
      <motion.div
        initial={{ opacity: 0, y: -8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.4 }}
        className="mb-6 flex items-start justify-between pt-8"
      >
        <div>
          <div className="flex items-center gap-3 mb-1">
            <h1 className="text-[22px] font-bold tracking-tight text-base-content">Models</h1>
            {totalModels > 0 && (
              <span className="inline-flex items-center px-2 py-0.5 rounded-full text-[10px] font-bold uppercase tracking-widest bg-base-200/70 text-base-content/40 border border-base-300/50">
                {totalModels}
              </span>
            )}
          </div>
          <p className="text-[13px] text-base-content/35">
            {totalModels === 0
              ? 'Add AI models to route requests through the gateway'
              : `${activeModels} active · ${totalModels - activeModels} disabled`}
          </p>
        </div>

        <AnimatePresence>
          {(providers?.length ?? 0) > 0 && (
            <motion.div
              initial={{ opacity: 0, scale: 0.92 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={{ opacity: 0, scale: 0.92 }}
              transition={{ duration: 0.2 }}
            >
              <Button
                icon={<Plus className="h-4 w-4" />}
                onClick={() => setIsAdding(true)}
                size="sm"
              >
                Add Model
              </Button>
            </motion.div>
          )}
        </AnimatePresence>
      </motion.div>

      {/* Stats strip */}
      {totalModels > 0 && (
        <motion.div
          initial={{ opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.4, delay: 0.1 }}
          className="grid grid-cols-3 gap-3 mb-7"
        >
          <div className="rounded-xl border border-base-300/50 bg-base-100/50 px-4 py-3">
            <div className="text-[10px] uppercase tracking-widest text-base-content/30 font-semibold mb-1">Total Models</div>
            <div className="text-[20px] font-bold text-base-content font-mono">{totalModels}</div>
          </div>
          <div className="rounded-xl border border-accent/20 bg-accent/5 px-4 py-3">
            <div className="text-[10px] uppercase tracking-widest text-accent/70 font-semibold mb-1">Active</div>
            <div className="text-[20px] font-bold text-accent font-mono">{activeModels}</div>
          </div>
          <div className="rounded-xl border border-base-300/50 bg-base-100/50 px-4 py-3">
            <div className="text-[10px] uppercase tracking-widest text-base-content/30 font-semibold mb-1">Disabled</div>
            <div className="text-[20px] font-bold text-base-content/50 font-mono">{totalModels - activeModels}</div>
          </div>
        </motion.div>
      )}

      {/* Model grid or empty state */}
      {totalModels === 0 ? (
        <EmptyState hasProviders={(providers?.length ?? 0) > 0} onAddClick={() => setIsAdding(true)} />
      ) : (
        <motion.div
          initial="hidden"
          animate="visible"
          variants={{ hidden: {}, visible: { transition: { staggerChildren: 0.05 } } }}
          className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4"
        >
          {models!.map((model, i) => (
            <ModelCard
              key={model.id}
              model={model}
              providerName={model.provider_name ?? providers?.find(p => p.id === model.provider_id)?.name ?? model.provider_id}
              index={i}
              onEdit={setEditingModel}
            />
          ))}
        </motion.div>
      )}

      <AddModelModal
        open={isAdding}
        onClose={() => setIsAdding(false)}
        providers={providers}
        onAdd={async (data) => {
          await createMutation.mutateAsync(data);
        }}
        isPending={createMutation.isPending}
      />

      <EditModelModal
        model={editingModel}
        open={editingModel !== null}
        onClose={() => setEditingModel(null)}
        onSave={async (data) => {
          if (!editingModel) return;
          await updateMutation.mutateAsync({
            providerId: editingModel.provider_id,
            modelName: editingModel.name,
            input: data,
          });
        }}
        isPending={updateMutation.isPending}
      />
    </div>
  );
}
