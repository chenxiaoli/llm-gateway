import { useState } from 'react';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z } from 'zod';
import { DollarSign, Plus, Cpu, Globe, Pencil, Trash2 } from 'lucide-react';
import { usePricingPolicies, useCreatePricingPolicy, useUpdatePricingPolicy, useDeletePricingPolicy } from '../hooks/usePricingPolicies';
import { Button } from '../components/ui/Button';
import { Badge } from '../components/ui/Badge';
import { Modal } from '../components/ui/Modal';
import { ConfirmDialog } from '../components/ui/ConfirmDialog';
import { motion } from 'framer-motion';
import type { PricingPolicyWithCounts } from '../types';
import type { PricingConfig, TierConfig, ContextTier } from '../types';

// ── Schema ────────────────────────────────────────────────────────────────────
const schema = z.object({
  name: z.string().min(1, 'Policy name is required'),
  billing_type: z.enum(['per_token', 'per_request', 'per_character', 'tiered_token', 'hybrid', 'context_tiered']),
  input_price_1m: z.string().optional(),
  output_price_1m: z.string().optional(),
  cache_read_price_1m: z.string().optional(),
  cache_creation_price_1m: z.string().optional(),
  request_price: z.string().optional(),
});

type FormValues = z.infer<typeof schema>;

const resolver = zodResolver(schema);

// Prices are stored as integer subunits (100_000_000 per USD) matching the backend.
const UNITS_PER_USD = 100_000_000;

function buildConfig(data: FormValues): PricingConfig {
  const { billing_type, input_price_1m, output_price_1m, cache_read_price_1m, cache_creation_price_1m, request_price } = data;
  const toSub = (v: string | undefined) => v ? Math.round(parseFloat(v) * UNITS_PER_USD) : undefined;
  switch (billing_type) {
    case 'per_token':
      return {
        input_price_1m: toSub(input_price_1m),
        output_price_1m: toSub(output_price_1m),
        cache_read_price_1m: toSub(cache_read_price_1m),
        cache_creation_price_1m: toSub(cache_creation_price_1m),
      };
    case 'per_request':
      return { request_price: toSub(request_price) };
    case 'per_character':
      return {
        input_price_1m: toSub(input_price_1m),
        output_price_1m: toSub(output_price_1m),
      };
    case 'tiered_token':
      return {
        input_price_1m: toSub(input_price_1m),
        output_price_1m: toSub(output_price_1m),
        cache_read_price_1m: toSub(cache_read_price_1m),
      };
    case 'hybrid':
      return {
        base_per_call: toSub(request_price),
        input_price_1m: toSub(input_price_1m),
        output_price_1m: toSub(output_price_1m),
        cache_read_price_1m: toSub(cache_read_price_1m),
        cache_creation_price_1m: toSub(cache_creation_price_1m),
      };
    case 'context_tiered':
      // Handled separately in PolicyFormModal via contextTiers state
      return { tiers: [] };
  }
}

const BILLING_TYPES: Record<string, string> = {
  per_token: 'Per Token',
  per_request: 'Per Request',
  per_character: 'Per Character',
  tiered_token: 'Tiered Token',
  context_tiered: 'Context Tiered',
  hybrid: 'Hybrid',
};

// ── Config renderer for table cells ───────────────────────────────────────────
function fmt(val: unknown): string {
  if (typeof val === 'number') return `$${(val / UNITS_PER_USD).toFixed(4)}/M`;
  if (typeof val === 'object' && val !== null) return JSON.stringify(val);
  return String(val);
}

function formatTokenCount(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(n % 1_000_000 === 0 ? 0 : 1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(n % 1_000 === 0 ? 0 : 1)}K`;
  return String(n);
}

function ConfigCell({ policy }: { policy: PricingPolicyWithCounts }) {
  const cfg = policy.config as Record<string, unknown>;
  const bt = policy.billing_type;

  if (bt === 'per_token' || bt === 'hybrid') {
    const input = cfg['input_price_1m'] as number | undefined;
    const output = cfg['output_price_1m'] as number | undefined;
    const cache = cfg['cache_read_price_1m'] as number | undefined;
    const cacheCreate = cfg['cache_creation_price_1m'] as number | undefined;
    const base = cfg['base_per_call'] as number | undefined;
    return (
      <div className="flex flex-wrap gap-1.5">
        {input != null && (
          <span className="inline-flex flex-col items-center px-2 py-1 rounded bg-base-200/60 border border-base-300/30 min-w-[64px]">
            <span className="text-[9px] font-bold uppercase tracking-wider text-base-content/30">In</span>
            <span className="text-xs font-mono font-bold text-base-content">{fmt(input)}</span>
          </span>
        )}
        {output != null && (
          <span className="inline-flex flex-col items-center px-2 py-1 rounded bg-base-200/60 border border-base-300/30 min-w-[64px]">
            <span className="text-[9px] font-bold uppercase tracking-wider text-base-content/30">Out</span>
            <span className="text-xs font-mono font-bold text-base-content">{fmt(output)}</span>
          </span>
        )}
        {cache != null && (
          <span className="inline-flex flex-col items-center px-2 py-1 rounded bg-base-200/60 border border-base-300/30 min-w-[64px]">
            <span className="text-[9px] font-bold uppercase tracking-wider text-base-content/30">Cache Read</span>
            <span className="text-xs font-mono font-bold text-base-content">{fmt(cache)}</span>
          </span>
        )}
        {cacheCreate != null && (
          <span className="inline-flex flex-col items-center px-2 py-1 rounded bg-base-200/60 border border-base-300/30 min-w-[64px]">
            <span className="text-[9px] font-bold uppercase tracking-wider text-base-content/30">Cache Create</span>
            <span className="text-xs font-mono font-bold text-base-content">{fmt(cacheCreate)}</span>
          </span>
        )}
        {bt === 'hybrid' && base != null && (
          <span className="inline-flex flex-col items-center px-2 py-1 rounded bg-base-200/60 border border-base-300/30 min-w-[64px]">
            <span className="text-[9px] font-bold uppercase tracking-wider text-base-content/30">Base</span>
            <span className="text-xs font-mono font-bold text-base-content">{fmt(base)}</span>
          </span>
        )}
      </div>
    );
  }

  if (bt === 'tiered_token') {
    const tiers = cfg['tiers'] as TierConfig[] | undefined;
    return (
      <span className="inline-flex items-center gap-1 text-xs text-base-content/40 italic">
        {tiers ? `${tiers.length} tier${tiers.length !== 1 ? 's' : ''}` : '—'}
      </span>
    );
  }

  if (bt === 'context_tiered') {
    const tiers = cfg['tiers'] as ContextTier[] | undefined;
    if (!tiers || tiers.length === 0) {
      return <span className="text-xs text-base-content/30 italic">—</span>;
    }
    return (
      <div className="flex flex-col gap-1">
        {tiers.map((tier, i) => {
          const label = tier.up_to != null ? `< ${formatTokenCount(tier.up_to)}` : `${formatTokenCount(0)}+`;
          return (
            <div key={i} className="flex items-center gap-1">
              <span className="text-[9px] font-bold uppercase tracking-wider text-base-content/30 min-w-[40px]">{label}</span>
              <span className="text-[10px] font-mono text-base-content/60">
                In {fmt(tier.input_price_1m)} · Out {fmt(tier.output_price_1m)}
              </span>
            </div>
          );
        })}
      </div>
    );
  }

  if (bt === 'per_request') {
    const p = cfg['request_price'] as number | undefined;
    return (
      <span className="inline-flex flex-col items-center px-2 py-1 rounded bg-base-200/60 border border-base-300/30">
        <span className="text-[9px] font-bold uppercase tracking-wider text-base-content/30">Per Call</span>
        <span className="text-xs font-mono font-bold text-base-content">{p != null ? fmt(p) : '—'}</span>
      </span>
    );
  }

  if (bt === 'per_character') {
    const input = cfg['input_price_1m'] as number | undefined;
    const output = cfg['output_price_1m'] as number | undefined;
    return (
      <div className="flex flex-wrap gap-1.5">
        {input != null && (
          <span className="inline-flex flex-col items-center px-2 py-1 rounded bg-base-200/60 border border-base-300/30 min-w-[64px]">
            <span className="text-[9px] font-bold uppercase tracking-wider text-base-content/30">In/Char</span>
            <span className="text-xs font-mono font-bold text-base-content">{fmt(input)}</span>
          </span>
        )}
        {output != null && (
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

// ── Context Tiered Fields (dynamic tier list) ──────────────────────────────────
const INPUT_CLASS = 'w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-primary/60 focus:ring-1 focus:ring-primary/20 transition-colors';

interface TierRow {
  up_to: string;
  input_price_1m: string;
  output_price_1m: string;
  cache_read_price_1m: string;
  cache_creation_price_1m: string;
}

function tierRowDefault(): TierRow {
  return { up_to: '', input_price_1m: '', output_price_1m: '', cache_read_price_1m: '', cache_creation_price_1m: '' };
}

function ContextTieredFields({ onChange, tiers: initialTiers }: { onChange: (tiers: TierRow[]) => void; tiers: TierRow[] }) {
  const [tiers, setTiers] = useState<TierRow[]>(initialTiers);

  const update = (updated: TierRow[]) => {
    setTiers(updated);
    onChange(updated);
  };

  const handleChange = (index: number, field: keyof TierRow, value: string) => {
    const next = [...tiers];
    next[index] = { ...next[index], [field]: value };
    update(next);
  };

  const addTier = () => update([...tiers, tierRowDefault()]);
  const removeTier = (index: number) => {
    if (tiers.length <= 1) return;
    update(tiers.filter((_, i) => i !== index));
  };

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50">
          Context Tiers
        </label>
        <button type="button" onClick={addTier} className="text-xs text-primary hover:text-primary/80 font-medium">
          + Add Tier
        </button>
      </div>
      {tiers.map((tier, i) => (
        <div key={i} className="p-3 rounded-lg border border-base-300/50 bg-base-200/30 space-y-2">
          <div className="flex items-center justify-between">
            <span className="text-[10px] font-bold uppercase tracking-wider text-base-content/40">Tier {i + 1}</span>
            {tiers.length > 1 && (
              <button type="button" onClick={() => removeTier(i)} className="text-[10px] text-error/60 hover:text-error">
                Remove
              </button>
            )}
          </div>
          <div className="grid grid-cols-2 gap-2">
            <div className="space-y-1">
              <label className="text-[10px] font-semibold uppercase tracking-wider text-base-content/40">
                {i < tiers.length - 1 ? 'Up To (tokens)' : 'Final Tier'}
              </label>
              {i < tiers.length - 1 ? (
                <input type="number" step="1" value={tier.up_to} onChange={e => handleChange(i, 'up_to', e.target.value)} placeholder="e.g. 32000" className={INPUT_CLASS} />
              ) : (
                <div className="h-10 flex items-center px-3 text-xs text-base-content/30 italic rounded-lg border border-base-300/30 bg-base-200/20">No upper limit</div>
              )}
            </div>
            <div />
            <div className="space-y-1">
              <label className="text-[10px] font-semibold uppercase tracking-wider text-base-content/40">Input ($/M)</label>
              <input type="number" step="0.0001" value={tier.input_price_1m} onChange={e => handleChange(i, 'input_price_1m', e.target.value)} placeholder="$0.00" className={INPUT_CLASS} />
            </div>
            <div className="space-y-1">
              <label className="text-[10px] font-semibold uppercase tracking-wider text-base-content/40">Output ($/M)</label>
              <input type="number" step="0.0001" value={tier.output_price_1m} onChange={e => handleChange(i, 'output_price_1m', e.target.value)} placeholder="$0.00" className={INPUT_CLASS} />
            </div>
            <div className="space-y-1">
              <label className="text-[10px] font-semibold uppercase tracking-wider text-base-content/40">Cache Read ($/M)</label>
              <input type="number" step="0.0001" value={tier.cache_read_price_1m} onChange={e => handleChange(i, 'cache_read_price_1m', e.target.value)} placeholder="$0.00" className={INPUT_CLASS} />
            </div>
            <div className="space-y-1">
              <label className="text-[10px] font-semibold uppercase tracking-wider text-base-content/40">Cache Create ($/M)</label>
              <input type="number" step="0.0001" value={tier.cache_creation_price_1m} onChange={e => handleChange(i, 'cache_creation_price_1m', e.target.value)} placeholder="$0.00" className={INPUT_CLASS} />
            </div>
          </div>
        </div>
      ))}
    </div>
  );
}

// ── Shared Policy Form Modal ──────────────────────────────────────────────────
interface PolicyFormModalProps {
  title: string;
  defaultValues?: Partial<PricingPolicyWithCounts>;
  onSubmit: (data: { name: string; billing_type: string; config: PricingConfig }) => Promise<void>;
  onClose: () => void;
  isPending: boolean;
}

function PolicyFormModal({ title, defaultValues, onSubmit, onClose, isPending }: PolicyFormModalProps) {
  const billingType = defaultValues?.billing_type ?? 'per_token';
  const cfg = defaultValues?.config as Record<string, unknown> | undefined;

  const {
    register,
    handleSubmit,
    watch,
    reset,
    formState: { errors },
  } = useForm<FormValues>({
    defaultValues: {
      name: defaultValues?.name ?? '',
      billing_type: (billingType as FormValues['billing_type']) ?? 'per_token',
      input_price_1m: cfg?.['input_price_1m'] != null ? String(Number(cfg['input_price_1m']) / UNITS_PER_USD) : '',
      output_price_1m: cfg?.['output_price_1m'] != null ? String(Number(cfg['output_price_1m']) / UNITS_PER_USD) : '',
      cache_read_price_1m: cfg?.['cache_read_price_1m'] != null ? String(Number(cfg['cache_read_price_1m']) / UNITS_PER_USD) : '',
      cache_creation_price_1m: cfg?.['cache_creation_price_1m'] != null ? String(Number(cfg['cache_creation_price_1m']) / UNITS_PER_USD) : '',
      request_price: cfg?.['request_price'] != null ? String(Number(cfg['request_price']) / UNITS_PER_USD) : '',
    },
    resolver,
  });

  const watchedBillingType = watch('billing_type');
  const [contextTiers, setContextTiers] = useState<TierRow[]>(() => {
    const tiersArr = cfg?.['tiers'] as Record<string, unknown>[] | undefined;
    if (tiersArr && tiersArr.length > 0) {
      return tiersArr.map(t => ({
        up_to: t['up_to'] != null ? String(t['up_to']) : '',
        input_price_1m: t['input_price_1m'] != null ? String(Number(t['input_price_1m']) / UNITS_PER_USD) : '',
        output_price_1m: t['output_price_1m'] != null ? String(Number(t['output_price_1m']) / UNITS_PER_USD) : '',
        cache_read_price_1m: t['cache_read_price_1m'] != null ? String(Number(t['cache_read_price_1m']) / UNITS_PER_USD) : '',
        cache_creation_price_1m: t['cache_creation_price_1m'] != null ? String(Number(t['cache_creation_price_1m']) / UNITS_PER_USD) : '',
      }));
    }
    return [tierRowDefault(), tierRowDefault()];
  });

  const onFormSubmit = async (data: FormValues) => {
    if (data.billing_type === 'context_tiered') {
      const toSub = (v: string) => v ? Math.round(parseFloat(v) * UNITS_PER_USD) : undefined;
      const tiers = contextTiers.map((tier, i) => ({
        up_to: i < contextTiers.length - 1 ? (tier.up_to ? parseInt(tier.up_to) : null) : null,
        input_price_1m: toSub(tier.input_price_1m),
        output_price_1m: toSub(tier.output_price_1m),
        cache_read_price_1m: toSub(tier.cache_read_price_1m),
        cache_creation_price_1m: toSub(tier.cache_creation_price_1m),
      }));
      await onSubmit({ name: data.name, billing_type: data.billing_type, config: { tiers } });
    } else {
      await onSubmit({
        name: data.name,
        billing_type: data.billing_type,
        config: buildConfig(data),
      });
    }
    reset();
    onClose();
  };

  return (
    <Modal open={true} onClose={onClose} title={title}>
      <form onSubmit={handleSubmit(onFormSubmit)} className="space-y-5">
        {/* Name */}
        <div className="space-y-1.5">
          <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50">
            Policy Name
          </label>
          <input
            type="text"
            {...register('name')}
            placeholder="e.g. Standard GPT-4 Pricing"
            className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-primary/60 focus:ring-1 focus:ring-primary/20 transition-colors"
          />
          {errors.name && (
            <p className="text-xs text-error mt-0.5">{errors.name.message}</p>
          )}
        </div>

        {/* Billing Type */}
        <div className="space-y-1.5">
          <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50">
            Billing Type
          </label>
          <select
            {...register('billing_type')}
            className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm text-base-content focus:outline-none focus:border-primary/60 focus:ring-1 focus:ring-primary/20 transition-colors"
          >
            {Object.entries(BILLING_TYPES).map(([val, label]) => (
              <option key={val} value={val}>{label}</option>
            ))}
          </select>
        </div>

        {/* Price fields by billing type */}
        {(watchedBillingType === 'per_token' || watchedBillingType === 'tiered_token') && (
          <div className="space-y-2">
            <div className="grid grid-cols-2 gap-2">
              <div className="space-y-1.5">
                <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50">
                  Input Price ($ per 1M)
                </label>
                <input
                  type="number"
                  step="0.0001"
                  {...register('input_price_1m')}
                  placeholder="$0.00"
                  className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-primary/60 focus:ring-1 focus:ring-primary/20 transition-colors"
                />
              </div>
              <div className="space-y-1.5">
                <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50">
                  Output Price ($ per 1M)
                </label>
                <input
                  type="number"
                  step="0.0001"
                  {...register('output_price_1m')}
                  placeholder="$0.00"
                  className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-primary/60 focus:ring-1 focus:ring-primary/20 transition-colors"
                />
              </div>
            </div>
            <div className="space-y-1.5">
              <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50">
                Cache Read Price ($ per 1M, cheaper)
              </label>
              <input
                type="number"
                step="0.0001"
                {...register('cache_read_price_1m')}
                placeholder="$0.00 (defaults to input price)"
                className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-primary/60 focus:ring-1 focus:ring-primary/20 transition-colors"
              />
            </div>
            <div className="space-y-1.5">
              <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50">
                Cache Creation Price ($ per 1M)
              </label>
              <input
                type="number"
                step="0.0001"
                {...register('cache_creation_price_1m')}
                placeholder="$0.00"
                className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-primary/60 focus:ring-1 focus:ring-primary/20 transition-colors"
              />
            </div>
          </div>
        )}

        {watchedBillingType === 'per_request' && (
          <div className="space-y-1.5">
            <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50">
              Per Request Price ($)
            </label>
            <input
              type="number"
              step="0.0001"
              {...register('request_price')}
              placeholder="$0.00"
              className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-primary/60 focus:ring-1 focus:ring-primary/20 transition-colors"
            />
          </div>
        )}

        {watchedBillingType === 'per_character' && (
          <div className="grid grid-cols-2 gap-2">
            <div className="space-y-1.5">
              <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50">
                Input Price ($ per 1M chars)
              </label>
              <input
                type="number"
                step="0.0001"
                {...register('input_price_1m')}
                placeholder="$0.00"
                className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-primary/60 focus:ring-1 focus:ring-primary/20 transition-colors"
              />
            </div>
            <div className="space-y-1.5">
              <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50">
                Output Price ($ per 1M chars)
              </label>
              <input
                type="number"
                step="0.0001"
                {...register('output_price_1m')}
                placeholder="$0.00"
                className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-primary/60 focus:ring-1 focus:ring-primary/20 transition-colors"
              />
            </div>
          </div>
        )}

        {watchedBillingType === 'hybrid' && (
          <div className="space-y-2">
            <div className="space-y-1.5">
              <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50">
                Base Price per Call ($)
              </label>
              <input
                type="number"
                step="0.0001"
                {...register('request_price')}
                placeholder="$0.00"
                className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-primary/60 focus:ring-1 focus:ring-primary/20 transition-colors"
              />
            </div>
            <div className="grid grid-cols-2 gap-2">
              <div className="space-y-1.5">
                <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50">
                  Input Price ($ per 1M)
                </label>
                <input
                  type="number"
                  step="0.0001"
                  {...register('input_price_1m')}
                  placeholder="$0.00"
                  className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-primary/60 focus:ring-1 focus:ring-primary/20 transition-colors"
                />
              </div>
              <div className="space-y-1.5">
                <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50">
                  Output Price ($ per 1M)
                </label>
                <input
                  type="number"
                  step="0.0001"
                  {...register('output_price_1m')}
                  placeholder="$0.00"
                  className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-primary/60 focus:ring-1 focus:ring-primary/20 transition-colors"
                />
              </div>
            </div>
            <div className="space-y-1.5">
              <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50">
                Cache Read Price ($ per 1M, cheaper)
              </label>
              <input
                type="number"
                step="0.0001"
                {...register('cache_read_price_1m')}
                placeholder="$0.00"
                className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-primary/60 focus:ring-1 focus:ring-primary/20 transition-colors"
              />
            </div>
            <div className="space-y-1.5">
              <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50">
                Cache Creation Price ($ per 1M)
              </label>
              <input
                type="number"
                step="0.0001"
                {...register('cache_creation_price_1m')}
                placeholder="$0.00"
                className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-primary/60 focus:ring-1 focus:ring-primary/20 transition-colors"
              />
            </div>
          </div>
        )}

        {watchedBillingType === 'context_tiered' && (
          <ContextTieredFields onChange={setContextTiers} tiers={contextTiers} />
        )}

        <div className="flex gap-2 pt-1">
          <Button type="submit" variant="primary" loading={isPending} className="flex-1">
            {title.includes('Edit') ? 'Save Changes' : 'Create Policy'}
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

      {isAdding && (
        <PolicyFormModal
          title="Add Pricing Policy"
          onSubmit={async (data) => {
            await createMutation.mutateAsync(data);
          }}
          onClose={() => setIsAdding(false)}
          isPending={createMutation.isPending}
        />
      )}

      {editingPolicy && (
        <PolicyFormModal
          title={`Edit ${editingPolicy.name}`}
          defaultValues={editingPolicy}
          onSubmit={async (data) => {
            await updateMutation.mutateAsync({ id: editingPolicy.id, input: data });
          }}
          onClose={() => setEditingPolicy(null)}
          isPending={updateMutation.isPending}
        />
      )}
    </div>
  );
}
