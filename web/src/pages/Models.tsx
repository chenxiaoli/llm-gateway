import { useState, useEffect } from 'react';
import { useAllModels, useCreateGlobalModel, useUpdateGlobalModel } from '../hooks/useModels';
import { usePricingPolicies } from '../hooks/usePricingPolicies';
import { Button } from '../components/ui/Button';
import { Modal } from '../components/ui/Modal';
import { Plus, Cpu, Pencil, Sparkles, Radio } from 'lucide-react';
import type { CreateGlobalModelRequest, ModelWithProvider, PricingPolicy } from '../types';
import { motion, AnimatePresence } from 'framer-motion';

// ── Price formatter ────────────────────────────────────────────────────────────
function formatPrice(centsPerMillion: number | undefined): string {
  if (centsPerMillion === undefined) return '—';
  return `$${(centsPerMillion / 1_000_000).toFixed(4)}/M`;
}

// ── Model Card ───────────────────────────────────────────────────────────────
interface ModelCardProps {
  model: ModelWithProvider;
  index: number;
  onEdit: (model: ModelWithProvider) => void;
  policies: PricingPolicy[];
}

function ModelCard({ model, index, onEdit, policies }: ModelCardProps) {
  const isActive = model.channel_names.length > 0;
  const policy = policies.find(p => p.id === model.pricing_policy_id);
  const billingType = policy?.billing_type ?? '';
  const config = policy?.config ?? {};
  const isPerToken = billingType === 'per_token';

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.45, delay: index * 0.05, ease: [0.25, 0.1, 0.25, 1] }}
      whileHover={{ y: -3 }}
      className="group relative"
    >
      <div className={`relative rounded-xl border bg-base-100/70 backdrop-blur-sm overflow-hidden transition-all duration-200 ${isActive ? 'border-accent/40 group-hover:bg-base-100/90' : 'bg-base-100/50'}`}>
        {/* Accent line top — only for active models */}
        <div className={`absolute top-0 left-0 right-0 h-[2px] transition-opacity duration-200 ${isActive ? 'bg-gradient-to-r from-accent to-accent/40 opacity-100' : 'opacity-0'}`} />

        <div className="p-5">
          {/* Header */}
          <div className="flex items-start justify-between mb-4">
            <div className="flex items-center gap-2.5">
              <div className={`w-8 h-8 rounded-lg flex items-center justify-center shrink-0 ${isActive ? 'bg-accent/10' : 'bg-base-200/70'}`}>
                <Cpu className={`h-4 w-4 ${isActive ? 'text-accent' : 'text-base-content/30'}`} />
              </div>
              <div>
                <div className="font-mono text-[13px] font-semibold text-base-content leading-tight">{model.name}</div>
              </div>
            </div>
            <div className={`flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[10px] font-semibold tracking-wide uppercase ${isActive ? 'bg-accent/10 text-accent' : 'bg-base-300/40 text-base-content/35'}`}>
              <span className={`w-1.5 h-1.5 rounded-full ${isActive ? 'bg-accent animate-pulse' : 'bg-base-content/25'}`} />
              {isActive ? 'Active' : 'Not routed'}
            </div>
          </div>

          {/* Pricing Policy */}
          <div className="flex items-start gap-2 px-3 py-2 rounded-lg bg-base-200/30 border border-base-300/20 mb-3">
            <div className="text-[10px] uppercase tracking-widest text-base-content/30 font-semibold shrink-0">Policy</div>
            {policy ? (
              <div className="flex flex-col gap-1">
                <span className="inline-flex items-center px-1.5 py-0.5 rounded bg-accent/8 text-accent/80 text-[10px] font-semibold border border-accent/15">
                  {policy.name}
                </span>
                {isPerToken ? (
                  <div className="flex flex-wrap gap-2">
                    <span className="flex items-center gap-0.5 text-[10px] text-base-content/50 font-mono">
                      <span className="text-[9px] text-base-content/25 uppercase">in</span>
                      {formatPrice(config['input_price'] as number | undefined)}
                    </span>
                    <span className="flex items-center gap-0.5 text-[10px] text-base-content/50 font-mono">
                      <span className="text-[9px] text-base-content/25 uppercase">out</span>
                      {formatPrice(config['output_price'] as number | undefined)}
                    </span>
                    <span className="flex items-center gap-0.5 text-[10px] text-base-content/50 font-mono">
                      <span className="text-[9px] text-base-content/25 uppercase">cache</span>
                      {formatPrice(config['cache_read_price'] as number | undefined)}
                    </span>
                  </div>
                ) : (
                  <span className="text-[10px] text-base-content/30 italic">{billingType}</span>
                )}
              </div>
            ) : (
              <span className="text-[11px] text-base-content/25 italic">No policy</span>
            )}
          </div>

          {/* Channels */}
          <div className="flex items-start gap-2 px-3 py-2 rounded-lg bg-base-200/30 border border-base-300/20 mb-4">
            <Radio className={`h-3 w-3 shrink-0 mt-0.5 ${isActive ? 'text-accent/60' : 'text-base-content/30'}`} />
            <div className="flex flex-wrap gap-1">
              {model.channel_names.length > 0 ? (
                model.channel_names.map((ch, i) => (
                  <span key={i} className="inline-flex items-center px-1.5 py-0.5 rounded bg-base-200/60 text-[10px] font-mono font-semibold text-base-content/60 border border-base-300/30">
                    {ch}
                  </span>
                ))
              ) : (
                <span className="text-[11px] text-base-content/25 italic">No channels</span>
              )}
            </div>
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
          </div>
        </div>
      </div>
    </motion.div>
  );
}

// ── Empty State ───────────────────────────────────────────────────────────────
function EmptyState({ onAddClick }: { onAddClick: () => void }) {
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
        Add your first AI model to start routing requests through the gateway.
      </p>
      <Button variant="primary" size="sm" onClick={onAddClick}>
        Add First Model
      </Button>
    </motion.div>
  );
}

// ── Add Model Modal ──────────────────────────────────────────────────────────

function AddModelModal({
  open,
  onClose,
  onAdd,
  isPending,
}: {
  open: boolean;
  onClose: () => void;
  onAdd: (data: CreateGlobalModelRequest) => Promise<void>;
  isPending: boolean;
}) {
  const { data: policies } = usePricingPolicies();
  const [name, setName] = useState('');
  const [pricingPolicyId, setPricingPolicyId] = useState('');

  const reset = () => { setName(''); setPricingPolicyId(''); };
  const handleClose = () => { reset(); onClose(); };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    await onAdd({
      name,
      pricing_policy_id: pricingPolicyId || undefined,
    });
    handleClose();
  };

  return (
    <Modal open={open} onClose={handleClose} title="Add Model">
      <form onSubmit={handleSubmit} className="space-y-5">
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

        {/* Pricing Policy */}
        <div className="space-y-1.5">
          <label className="text-[11px] font-semibold uppercase tracking-wider text-base-content/50">Pricing Policy</label>
          <select
            value={pricingPolicyId}
            onChange={(e) => setPricingPolicyId(e.target.value)}
            className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-[13px] text-base-content focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
          >
            <option value="">No policy (pricing handled at channel level)</option>
            {policies?.map((p) => (
              <option key={p.id} value={p.id}>{p.name}</option>
            ))}
          </select>
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
  onSave: (data: { pricing_policy_id?: string | null }) => Promise<void>;
  isPending: boolean;
}) {
  const { data: policies } = usePricingPolicies();
  const [pricingPolicyId, setPricingPolicyId] = useState('');

  // Sync state when model changes
  useEffect(() => {
    if (model) {
      setPricingPolicyId(model.pricing_policy_id ?? '');
    }
  }, [model]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    await onSave({
      pricing_policy_id: pricingPolicyId || undefined,
    });
    onClose();
  };

  return (
    <Modal open={open} onClose={onClose} title={`Edit ${model?.name ?? 'Model'}`}>
      <form onSubmit={handleSubmit} className="space-y-5">
        {/* Pricing Policy */}
        <div className="space-y-1.5">
          <label className="text-[11px] font-semibold uppercase tracking-wider text-base-content/50">Pricing Policy</label>
          <select
            value={pricingPolicyId}
            onChange={(e) => setPricingPolicyId(e.target.value)}
            className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-[13px] text-base-content focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
          >
            <option value="">No policy</option>
            {policies?.map((p) => (
              <option key={p.id} value={p.id}>{p.name}</option>
            ))}
          </select>
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
  const createMutation = useCreateGlobalModel();
  const updateMutation = useUpdateGlobalModel();
  const [isAdding, setIsAdding] = useState(false);
  const [editingModel, setEditingModel] = useState<ModelWithProvider | null>(null);
  const { data: policies } = usePricingPolicies();

  const totalModels = models?.length ?? 0;
  const activeModels = models?.filter(m => m.channel_names.length > 0).length ?? 0;

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
              : `${activeModels} active · ${totalModels - activeModels} not routed`}
          </p>
        </div>

        <AnimatePresence>
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
        </AnimatePresence>
      </motion.div>

      {/* Model grid or empty state */}
      {totalModels === 0 ? (
        <EmptyState onAddClick={() => setIsAdding(true)} />
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
              index={i}
              onEdit={setEditingModel}
              policies={policies ?? []}
            />
          ))}
        </motion.div>
      )}

      <AddModelModal
        open={isAdding}
        onClose={() => setIsAdding(false)}
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
            modelName: editingModel.name,
            input: data,
          });
        }}
        isPending={updateMutation.isPending}
      />
    </div>
  );
}
