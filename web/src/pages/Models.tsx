import { useState, useEffect } from 'react';
import { useAllModels, useCreateGlobalModel, useUpdateGlobalModel } from '../hooks/useModels';
import { usePricingPolicies } from '../hooks/usePricingPolicies';
import { Button } from '../components/ui/Button';
import { Modal } from '../components/ui/Modal';
import { Plus, Cpu, Pencil, Sparkles, Radio } from 'lucide-react';
import type { CreateGlobalModelRequest, ModelWithProvider, PricingPolicy } from '../types';
import { motion } from 'framer-motion';

// ── Price formatter ────────────────────────────────────────────────────────────
function formatPrice(dollarsPerMillion: number | undefined): string {
  if (dollarsPerMillion === undefined) return '—';
  return `$${dollarsPerMillion.toFixed(4)}`;
}

// ── Page-level stat pill ──────────────────────────────────────────────────────
interface StatPillProps {
  label: string;
  value: string | number;
  accent?: boolean;
}
function StatPill({ label, value, accent }: StatPillProps) {
  return (
    <div className={`flex items-center gap-3 px-4 py-2.5 rounded-xl border backdrop-blur-sm transition-all duration-200 ${
      accent
        ? 'bg-accent/[0.07] border-accent/20'
        : 'bg-base-100/40 border-base-300/40'
    }`}>
      <span className={`text-xs font-semibold uppercase tracking-widest ${accent ? 'text-accent/70' : 'text-base-content/30'}`}>
        {label}
      </span>
      <span className={`text-xl font-bold tracking-tight font-mono ${accent ? 'text-accent' : 'text-base-content'}`}>
        {value}
      </span>
    </div>
  );
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
      initial={{ opacity: 0, y: 16 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.5, delay: 0.1 + index * 0.06, ease: [0.16, 1, 0.3, 1] }}
      className="group relative"
    >
      <div className={`
        relative rounded-2xl overflow-hidden transition-all duration-300 cursor-pointer
        ${isActive
          ? 'bg-gradient-to-b from-base-100 to-base-100/60 border border-accent/20 group-hover:border-accent/40 group-hover:shadow-[0_0_24px_-4px_rgba(var(--accent),0.12)]'
          : 'bg-base-100/40 border border-base-300/30 group-hover:border-base-300/60 group-hover:bg-base-100/70'
        }
        group-hover:-translate-y-0.5
      `}>
        {/* Top accent bar */}
        <div className={`h-[2px] w-full ${isActive ? 'bg-gradient-to-r from-accent via-accent/60 to-transparent' : 'bg-gradient-to-r from-base-300/40 to-transparent'}`} />

        {/* Scanline overlay for active cards */}
        {isActive && (
          <div className="absolute inset-0 bg-[linear-gradient(180deg,transparent_0%,rgba(var(--accent),0.015)_50%,transparent_100%)] pointer-events-none" />
        )}

        <div className="relative p-5">
          {/* Header row */}
          <div className="flex items-start justify-between mb-5">
            <div className="flex items-center gap-3">
              {/* Icon container */}
              <div className={`
                relative w-10 h-10 rounded-xl flex items-center justify-center shrink-0
                transition-all duration-300
                ${isActive
                  ? 'bg-accent/10 ring-1 ring-accent/20'
                  : 'bg-base-200/60'
                }
              `}>
                <Cpu className={`h-5 w-5 transition-colors duration-300 ${isActive ? 'text-accent' : 'text-base-content/25'}`} />
                {/* Active pulse dot */}
                {isActive && (
                  <span className="absolute -top-0.5 -right-0.5 w-2 h-2 rounded-full bg-accent animate-pulse" />
                )}
              </div>

              {/* Model name */}
              <div className="min-w-0">
                <div className="font-mono text-lg font-bold text-base-content leading-tight truncate max-w-[200px]">
                  {model.name}
                </div>
                <div className={`text-xs mt-0.5 ${isActive ? 'text-accent/50' : 'text-base-content/20'}`}>
                  {isActive ? `${model.channel_names.length} channel${model.channel_names.length !== 1 ? 's' : ''} active` : 'No routing'}
                </div>
              </div>
            </div>

            {/* Status badge */}
            <div className={`
              shrink-0 flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-bold uppercase tracking-widest border
              transition-all duration-300
              ${isActive
                ? 'bg-accent/10 text-accent border-accent/25'
                : 'bg-base-200/40 text-base-content/25 border-base-300/40'
              }
            `}>
              <span className={`w-1.5 h-1.5 rounded-full ${isActive ? 'bg-accent animate-pulse' : 'bg-base-content/20'}`} />
              {isActive ? 'Live' : 'Idle'}
            </div>
          </div>

          {/* Divider */}
          <div className="h-px bg-gradient-to-r from-base-300/20 via-base-300/10 to-transparent mb-5" />

          {/* Pricing section */}
          <div className="mb-5">
            <div className="flex items-center gap-2 mb-3">
              <div className={`text-xs font-black uppercase tracking-[0.2em] ${isActive ? 'text-accent/60' : 'text-base-content/25'}`}>
                Pricing
              </div>
              <div className={`flex-1 h-px ${isActive ? 'bg-gradient-to-r from-accent/20 to-transparent' : 'bg-base-300/20'}`} />
            </div>

            {policy ? (
              <div className="space-y-2">
                {/* Policy name tag */}
                <div className="flex items-center gap-2">
                  <span className={`
                    inline-flex items-center px-2 py-0.5 rounded-md text-sm font-bold border
                    ${isActive
                      ? 'bg-accent/10 text-accent/90 border-accent/25'
                      : 'bg-base-200/50 text-base-content/50 border-base-300/40'
                    }
                  `}>
                    {policy.name}
                  </span>
                  {isPerToken && (
                    <span className={`text-xs font-semibold uppercase tracking-wider ${isActive ? 'text-accent/40' : 'text-base-content/20'}`}>
                      per 1M tokens
                    </span>
                  )}
                </div>

                {/* Per-token price grid */}
                {isPerToken ? (
                  <div className={`
                    grid grid-cols-3 gap-1.5 p-2.5 rounded-xl border
                    ${isActive ? 'bg-base-100/50 border-accent/10' : 'bg-base-200/20 border-base-300/20'}
                  `}>
                    {[
                      { label: 'Input', keys: ['input_per_1m', 'input_price'] },
                      { label: 'Output', keys: ['output_per_1m', 'output_price'] },
                      { label: 'Cache', keys: ['cache_read_price'] },
                    ].map(({ label, keys }) => {
                      const val = (keys.length > 1
                        ? (config[keys[0]] ?? config[keys[1]])
                        : config[keys[0]]) as number | undefined;
                      return (
                        <div key={label} className="flex flex-col items-center text-center py-1">
                          <span className={`text-xs font-bold uppercase tracking-wider mb-1 ${isActive ? 'text-accent/50' : 'text-base-content/25'}`}>
                            {label}
                          </span>
                          <span className={`font-mono text-lg font-bold ${isActive ? 'text-accent' : 'text-base-content/60'}`}>
                            {formatPrice(val)}
                          </span>
                        </div>
                      );
                    })}
                  </div>
                ) : (
                  <div className="flex items-center gap-2">
                    <span className={`text-sm font-mono ${isActive ? 'text-accent/70' : 'text-base-content/30'}`}>
                      {billingType}
                    </span>
                  </div>
                )}
              </div>
            ) : (
              <div className="flex items-center gap-1.5">
                <div className={`w-1 h-1 rounded-full ${isActive ? 'bg-accent/30' : 'bg-base-content/15'}`} />
                <span className={`text-sm italic ${isActive ? 'text-accent/30' : 'text-base-content/20'}`}>
                  No policy — channel-level pricing
                </span>
              </div>
            )}
          </div>

          {/* Channels section */}
          {model.channel_names.length > 0 && (
            <div className="mb-5">
              <div className="flex items-center gap-2 mb-3">
                <div className={`text-xs font-black uppercase tracking-[0.2em] ${isActive ? 'text-accent/60' : 'text-base-content/25'}`}>
                  Channels
                </div>
                <div className={`flex-1 h-px ${isActive ? 'bg-gradient-to-r from-accent/20 to-transparent' : 'bg-base-300/20'}`} />
              </div>

              <div className="flex flex-wrap gap-1.5">
                {model.channel_names.map((ch, i) => (
                  <span
                    key={i}
                    className={`
                      inline-flex items-center gap-1 px-2 py-1 rounded-md text-sm font-mono font-bold border
                      transition-all duration-200
                      ${isActive
                        ? 'bg-base-100/60 text-base-content/70 border-base-300/40 hover:border-accent/30 hover:text-accent/70'
                        : 'bg-base-200/40 text-base-content/35 border-base-300/30'
                      }
                    `}
                  >
                    <Radio className={`h-3 w-3 shrink-0 ${isActive ? 'text-accent/50' : 'text-base-content/20'}`} />
                    {ch}
                  </span>
                ))}
              </div>
            </div>
          )}

          {/* Footer */}
          <div className="flex items-center justify-between pt-4 border-t border-base-300/20">
            <div className={`text-xs font-mono uppercase tracking-wider ${isActive ? 'text-accent/30' : 'text-base-content/15'}`}>
              {model.id}
            </div>
            <button
              onClick={() => onEdit(model)}
              className={`
                flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-sm font-medium border
                transition-all duration-200 cursor-pointer
                ${isActive
                  ? 'text-base-content/40 border-base-300/30 hover:border-accent/40 hover:text-accent'
                  : 'text-base-content/25 border-base-300/20 hover:border-base-300/50 hover:text-base-content/50'
                }
              `}
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
      transition={{ duration: 0.6, ease: [0.16, 1, 0.3, 1] }}
      className="flex flex-col items-center justify-center py-32 px-4"
    >
      {/* Decorative grid */}
      <div className="relative mb-10">
        <div className="w-20 h-20 rounded-3xl border-2 border-base-300/30 flex items-center justify-center bg-base-100/30 backdrop-blur-sm">
          <Cpu className="h-10 w-10 text-base-content/15" />
        </div>
        {/* Corner accents */}
        <div className="absolute -top-1 -left-1 w-3 h-3 border-t-2 border-l-2 border-accent/30 rounded-tl-lg" />
        <div className="absolute -bottom-1 -right-1 w-3 h-3 border-b-2 border-r-2 border-accent/30 rounded-br-lg" />
        {/* Plus icon */}
        <div className="absolute -bottom-2 -right-2 w-7 h-7 rounded-xl bg-base-100 border-2 border-base-300/40 flex items-center justify-center">
          <Plus className="h-3.5 w-3.5 text-accent/60" />
        </div>
      </div>

      <div className="text-center max-w-sm">
        <h3 className="text-lg font-bold text-base-content/50 mb-2">No models registered</h3>
        <p className="text-sm text-base-content/25 leading-relaxed mb-8">
          Add AI models to enable request routing through the gateway. Each model can be associated with a pricing policy.
        </p>
        <Button variant="primary" size="sm" onClick={onAddClick}>
          Register First Model
        </Button>
      </div>
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
    <Modal open={open} onClose={handleClose} title="Register Model">
      <form onSubmit={handleSubmit} className="space-y-5">
        <div className="space-y-1.5">
          <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Model Name</label>
          <div className="relative">
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              required
              placeholder="e.g. gpt-4o, claude-3-5-sonnet"
              className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 pl-9 pr-3 text-sm font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
            />
            <Sparkles className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-base-content/25" />
          </div>
        </div>

        <div className="space-y-1.5">
          <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Pricing Policy</label>
          <select
            value={pricingPolicyId}
            onChange={(e) => setPricingPolicyId(e.target.value)}
            className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm text-base-content focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
          >
            <option value="">No policy (pricing handled at channel level)</option>
            {policies?.map((p) => (
              <option key={p.id} value={p.id}>{p.name}</option>
            ))}
          </select>
        </div>

        <div className="flex items-center gap-2 pt-1">
          <Button type="submit" variant="primary" loading={isPending} className="flex-1">
            Register
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
        <div className="space-y-1.5">
          <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Pricing Policy</label>
          <select
            value={pricingPolicyId}
            onChange={(e) => setPricingPolicyId(e.target.value)}
            className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm text-base-content focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
          >
            <option value="">No policy</option>
            {policies?.map((p) => (
              <option key={p.id} value={p.id}>{p.name}</option>
            ))}
          </select>
        </div>

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
  const totalPolicies = new Set(models?.map(m => m.pricing_policy_id).filter(Boolean)).size;

  if (isLoading) {
    return (
      <div className="px-6 pb-8 pt-8">
        {/* Header skeleton */}
        <div className="mb-8 flex items-start justify-between">
          <div className="space-y-2">
            <div className="h-7 w-24 bg-base-200/60 rounded-lg animate-pulse" />
            <div className="h-4 w-48 bg-base-200/40 rounded animate-pulse" />
          </div>
          <div className="h-9 w-28 bg-base-200/40 rounded-lg animate-pulse" />
        </div>
        {/* Card skeletons */}
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
          {[...Array(6)].map((_, i) => (
            <div key={i} className="h-56 bg-base-100/30 rounded-2xl border border-base-300/20 animate-pulse" />
          ))}
        </div>
      </div>
    );
  }

  return (
    <div className="px-6 pb-8">
      {/* ── Hero header ── */}
      <motion.div
        initial={{ opacity: 0, y: -12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.5, ease: [0.16, 1, 0.3, 1] }}
        className="mb-8 pt-8"
      >
        <div className="flex items-start justify-between">
          <div>
            <h1 className="text-3xl font-black tracking-tight text-base-content leading-none mb-1">
              Models
            </h1>
            <p className="text-sm text-base-content/30">
              {totalModels === 0
                ? 'Register AI models to route requests through the gateway'
                : `${activeModels} live · ${totalModels - activeModels} idle · ${totalPolicies} policy${totalPolicies !== 1 ? 'ies' : ''}`
              }
            </p>
          </div>

          <Button
            icon={<Plus className="h-4 w-4" />}
            onClick={() => setIsAdding(true)}
            size="sm"
          >
            Add Model
          </Button>
        </div>

        {/* Stats row */}
        {totalModels > 0 && (
          <motion.div
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.4, delay: 0.1, ease: [0.16, 1, 0.3, 1] }}
            className="flex flex-wrap gap-2.5 mt-6"
          >
            <StatPill label="Total" value={totalModels} />
            <StatPill label="Live" value={activeModels} accent />
            <StatPill label="Idle" value={totalModels - activeModels} />
          </motion.div>
        )}
      </motion.div>

      {/* ── Grid ── */}
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
