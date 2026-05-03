import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';
import { Cpu, Search } from 'lucide-react';
import { useUserModels } from '../hooks/useUserModels';
import { useReducedMotion } from '../hooks/useReducedMotion';
import type { UserModelView, ContextTier } from '../types';

const UNITS_PER_USD = 100_000_000;

function formatPrice(val: number | undefined): string {
  if (val === undefined || val === null) return '—';
  return `$${(val / UNITS_PER_USD).toFixed(2)}`;
}

function formatTokenCount(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(n % 1_000_000 === 0 ? 0 : 1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(n % 1_000 === 0 ? 0 : 1)}K`;
  return String(n);
}

function StatPill({ label, value, accent }: { label: string; value: number; accent?: boolean }) {
  return (
    <div className={`inline-flex items-center gap-2 px-3 py-1.5 rounded-lg border text-sm font-mono ${
      accent
        ? 'bg-accent/5 border-accent/20 text-accent'
        : 'bg-base-200/40 border-base-300/40 text-base-content/60'
    }`}>
      <span className="text-xs font-bold uppercase tracking-wider opacity-60">{label}</span>
      <span className="font-bold">{value}</span>
    </div>
  );
}

function ConsoleModelCard({ model, index, reducedMotion }: { model: UserModelView; index: number; reducedMotion: boolean }) {
  const { t } = useTranslation();
  const policy = model.pricing;
  const billingType = policy?.billing_type ?? '';
  const config = (policy?.config ?? {}) as Record<string, unknown>;
  const isPerToken = billingType === 'per_token';
  const isContextTiered = billingType === 'context_tiered';

  return (
    <motion.div
      initial={reducedMotion ? false : { opacity: 0, y: 16 }}
      animate={{ opacity: 1, y: 0 }}
      transition={reducedMotion ? { duration: 0 } : { duration: 0.4, delay: 0.05 + Math.min(index, 12) * 0.04, ease: [0.16, 1, 0.3, 1] }}
    >
      <div className="relative rounded-2xl overflow-hidden transition-all duration-300 bg-base-100 border border-base-300/50 hover:border-accent/30 hover:shadow-[0_0_24px_-4px_rgba(var(--accent),0.08)] hover:-translate-y-0.5">
        <div className="absolute left-0 top-0 bottom-0 w-[3px] bg-accent/60 rounded-l-2xl" />

        <div className="relative p-5">
          {/* Header */}
          <div className="flex items-start justify-between mb-4">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 rounded-xl flex items-center justify-center shrink-0 bg-accent/10">
                <Cpu className="h-5 w-5 text-accent" />
              </div>
              <div className="min-w-0">
                <div className="font-mono text-lg font-bold text-base-content leading-tight truncate max-w-[200px]" title={model.name}>
                  {model.name}
                </div>
                {model.model_type && (
                  <div className="text-xs mt-0.5 text-base-content/50">{model.model_type}</div>
                )}
              </div>
            </div>
          </div>

          {/* Divider */}
          <div className="h-px bg-base-300/20 mb-4" />

          {/* Pricing section */}
          <div>
            <div className="flex items-center gap-2 mb-3">
              <div className="text-xs font-bold uppercase tracking-widest text-base-content/50">{t('consoleModels.card.pricing')}</div>
              <div className="flex-1 h-px bg-base-300/20" />
            </div>

            {policy ? (
              <div className="space-y-2">
                <div className="flex items-center gap-2">
                  <span className="inline-flex items-center px-2 py-0.5 rounded-md text-sm font-semibold border bg-base-200/50 text-base-content/70 border-base-300/40">
                    {model.pricing_policy_name}
                  </span>
                  {(isPerToken || isContextTiered) && (
                    <span className="text-xs text-base-content/40">{t('consoleModels.card.per1mTokens')}</span>
                  )}
                </div>

                {isPerToken ? (
                  <div className="grid grid-cols-3 gap-1.5 p-2.5 rounded-xl border bg-base-200/20 border-base-300/20">
                    {[
                      { label: t('consoleModels.card.input'), key: 'input_price_1m' },
                      { label: t('consoleModels.card.output'), key: 'output_price_1m' },
                      { label: t('consoleModels.card.cache'), key: 'cache_read_price_1m' },
                    ].map(({ label, key }) => {
                      const val = config[key] as number | undefined;
                      return (
                        <div key={key} className="flex flex-col items-center text-center py-1">
                          <span className="text-xs font-semibold text-base-content/40 mb-1">{label}</span>
                          <span className="font-mono text-lg font-bold text-base-content">
                            {formatPrice(val)}
                          </span>
                        </div>
                      );
                    })}
                  </div>
                ) : isContextTiered ? (
                  <div className="space-y-1.5">
                    {(config['tiers'] as ContextTier[] | undefined)?.map((tier, i, tiers) => {
                      const label = tier.up_to != null
                        ? `< ${formatTokenCount(tier.up_to)}`
                        : `> ${i > 0 && tiers[i - 1].up_to != null ? formatTokenCount(tiers[i - 1].up_to!) : '0'}`;
                      return (
                        <div key={i} className="p-2 rounded-xl border bg-base-200/20 border-base-300/20">
                          <div className="text-xs font-semibold uppercase tracking-wider text-base-content/40 mb-1.5">{label} ctx</div>
                          <div className="grid grid-cols-3 gap-1.5">
                            <div className="flex flex-col items-center text-center">
                              <span className="text-xs font-semibold text-base-content/40 mb-0.5">{t('consoleModels.card.input')}</span>
                              <span className="font-mono text-sm font-bold text-base-content">{formatPrice(tier.input_price_1m)}</span>
                            </div>
                            <div className="flex flex-col items-center text-center">
                              <span className="text-xs font-semibold text-base-content/40 mb-0.5">{t('consoleModels.card.output')}</span>
                              <span className="font-mono text-sm font-bold text-base-content">{formatPrice(tier.output_price_1m)}</span>
                            </div>
                            <div className="flex flex-col items-center text-center">
                              <span className="text-xs font-semibold text-base-content/40 mb-0.5">{t('consoleModels.card.cache')}</span>
                              <span className="font-mono text-sm font-bold text-base-content">{formatPrice(tier.cache_read_price_1m)}</span>
                            </div>
                          </div>
                        </div>
                      );
                    }) ?? (
                      <div className="text-sm italic text-base-content/40">{t('consoleModels.card.noTiers')}</div>
                    )}
                  </div>
                ) : (
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-mono text-base-content/60">{billingType}</span>
                  </div>
                )}
              </div>
            ) : (
              <div className="flex items-center gap-1.5">
                <div className="w-1 h-1 rounded-full bg-base-content/25" />
                <span className="text-sm italic text-base-content/40">{t('consoleModels.card.noPricingPolicy')}</span>
              </div>
            )}
          </div>
        </div>
      </div>
    </motion.div>
  );
}

export default function ConsoleModels() {
  const { t } = useTranslation();
  const { data: models, isLoading } = useUserModels();
  const [search, setSearch] = useState('');
  const reducedMotion = useReducedMotion();

  const filtered = models?.filter(m =>
    m.name.toLowerCase().includes(search.toLowerCase())
  ) ?? [];

  const totalModels = models?.length ?? 0;

  if (isLoading) {
    return (
      <div className="px-6 pb-8">
        <div className="mb-8 pt-8">
          <div className="space-y-2">
            <div className="h-7 w-24 bg-base-200/60 rounded-lg animate-pulse" />
            <div className="h-4 w-48 bg-base-200/40 rounded animate-pulse" />
          </div>
        </div>
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
      {/* Header */}
      <motion.div
        initial={reducedMotion ? false : { opacity: 0, y: -12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={reducedMotion ? { duration: 0 } : { duration: 0.5, ease: [0.16, 1, 0.3, 1] }}
        className="mb-8 pt-8"
      >
        <div className="flex items-start justify-between">
          <div>
            <h1 className="text-3xl font-black tracking-tight text-base-content leading-none mb-1">
              {t('consoleModels.title')}
            </h1>
            <p className="text-base text-base-content/50">
              {totalModels === 0
                ? t('consoleModels.description')
                : t('consoleModels.descriptionCount', { count: totalModels })
              }
            </p>
          </div>

          {/* Search */}
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-base-content/40" />
            <input
              type="text"
              placeholder={t('consoleModels.searchPlaceholder')}
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="input input-sm input-bordered pl-9 w-56 bg-base-200/40 border-base-300/40 focus:border-accent/40 focus:outline-none"
            />
          </div>
        </div>

        {/* Stats row */}
        {totalModels > 0 && (
          <motion.div
            initial={reducedMotion ? false : { opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            transition={reducedMotion ? { duration: 0 } : { duration: 0.4, delay: 0.1, ease: [0.16, 1, 0.3, 1] }}
            className="flex flex-wrap gap-2.5 mt-6"
          >
            <StatPill label={t('consoleModels.stats.available')} value={totalModels} accent />
          </motion.div>
        )}
      </motion.div>

      {/* Empty state */}
      {filtered.length === 0 && totalModels > 0 && (
        <div className="flex flex-col items-center justify-center py-20 text-center">
          <Search className="h-10 w-10 text-base-content/20 mb-4" />
          <p className="text-base-content/40">{t('consoleModels.noMatch', { search })}</p>
        </div>
      )}

      {/* Grid */}
      {filtered.length > 0 && (
        <motion.div
          initial="hidden"
          animate="visible"
          variants={{ hidden: {}, visible: { transition: { staggerChildren: 0.05 } } }}
          className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4"
        >
          {filtered.map((model, i) => (
            <ConsoleModelCard
              key={model.name}
              model={model}
              index={i}
              reducedMotion={reducedMotion}
            />
          ))}
        </motion.div>
      )}
    </div>
  );
}
