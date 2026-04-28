import { useState } from 'react';
import { DollarSign, MessageSquare, Zap, ArrowDownToLine, ArrowUpFromLine, BarChart3, Filter, RotateCcw, Clock } from 'lucide-react';
import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer } from 'recharts';
import { motion } from 'framer-motion';
import { useUsage, useUsageSummary } from '../hooks/useUsage';
import { useKeys } from '../hooks/useKeys';
import { useReducedMotion } from '../hooks/useReducedMotion';
import { Button } from '../components/ui/Button';
import { Badge } from '../components/ui/Badge';

const EASE = [0.16, 1, 0.3, 1] as const;

interface MetricCardProps {
  label: string;
  value: string;
  icon: React.ReactNode;
  index: number;
  reducedMotion: boolean;
}

function MetricCard({ label, value, icon, index, reducedMotion }: MetricCardProps) {
  return (
    <motion.div
      initial={reducedMotion ? false : { opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={reducedMotion ? { duration: 0 } : { duration: 0.35, delay: index * 0.05, ease: EASE }}
      className="relative rounded-2xl border border-base-300/40 bg-base-100 p-4 sm:p-5 overflow-hidden"
    >
      <div className="flex items-center justify-between mb-3">
        <span className="text-[11px] font-semibold uppercase tracking-wider text-base-content/50">{label}</span>
        <div className="w-7 h-7 rounded-lg flex items-center justify-center bg-base-200/60 shrink-0">
          {icon}
        </div>
      </div>
      <div className="font-mono text-lg sm:text-2xl font-bold tracking-tight leading-tight break-all">{value}</div>
    </motion.div>
  );
}

export default function Usage() {
  const [since, setSince] = useState('');
  const [until, setUntil] = useState('');
  const [keyFilter, setKeyFilter] = useState('');
  const [page, setPage] = useState(1);
  const [pageSize] = useState(20);
  const reducedMotion = useReducedMotion();

  const filter = { since: since || undefined, until: until || undefined, key_id: keyFilter || undefined };

  const { data, isLoading } = useUsage(filter, page, pageSize);
  const { data: summaryData, isLoading: summaryLoading } = useUsageSummary(filter);
  const { data: keys } = useKeys();

  const usageItems = data?.items ?? [];
  const totalPages = Math.ceil((data?.total ?? 0) / pageSize);

  const summary = summaryData ?? [];
  const grandTotals = summary.reduce<{ cost: number; requests: number; inputTokens: number; cacheReadTokens: number; cacheCreationTokens: number; outputTokens: number }>(
    (acc, r) => ({
      cost: acc.cost + r.total_cost,
      requests: acc.requests + r.request_count,
      inputTokens: acc.inputTokens + r.total_input_tokens,
      cacheReadTokens: acc.cacheReadTokens + r.total_cache_read_tokens,
      cacheCreationTokens: acc.cacheCreationTokens + r.total_cache_creation_tokens,
      outputTokens: acc.outputTokens + r.total_output_tokens,
    }),
    { cost: 0, requests: 0, inputTokens: 0, cacheReadTokens: 0, cacheCreationTokens: 0, outputTokens: 0 }
  );

  const chartData = summary.map((r) => ({
    model: r.model_name,
    input: r.total_input_tokens,
    cache: r.total_cache_read_tokens,
    cacheCreation: r.total_cache_creation_tokens,
    output: r.total_output_tokens,
    cost: r.total_cost,
    requests: r.request_count,
  }));

  const inputStyle = "h-9 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm text-base-content placeholder:text-base-content/25 focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors";
  const selectStyle = "h-9 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm text-base-content focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors cursor-pointer";

  return (
    <div className="px-6 pb-8">
      {/* Header */}
      <motion.div
        initial={reducedMotion ? false : { opacity: 0, y: -12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={reducedMotion ? { duration: 0 } : { duration: 0.4, ease: EASE }}
        className="mb-6 pt-8 flex items-center justify-between gap-4"
      >
        <div>
          <h1 className="text-3xl font-black tracking-tight text-base-content leading-none mb-1">
            Usage
          </h1>
          <p className="text-base text-base-content/50">
            Token consumption and cost breakdown
          </p>
        </div>
        {!isLoading && data && (
          <div className="hidden sm:flex items-center gap-2 px-4 py-2.5 rounded-xl border border-base-300/40 bg-base-100 shrink-0">
            <BarChart3 className="h-4 w-4 text-base-content/40" />
            <span className="font-mono text-sm font-medium">{data.total.toLocaleString()}</span>
            <span className="text-xs text-base-content/40">records</span>
          </div>
        )}
      </motion.div>

      {/* Filters */}
      <motion.div
        initial={reducedMotion ? false : { opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={reducedMotion ? { duration: 0 } : { duration: 0.35, delay: 0.05, ease: EASE }}
        className="mb-5"
      >
        <div className="rounded-2xl border border-base-300/40 bg-base-100 overflow-hidden">
          <div className="px-5 py-3 border-b border-base-300/60 bg-base-100/60 flex items-center justify-between">
            <span className="text-[10px] font-mono font-semibold uppercase tracking-[0.18em] text-base-content/25 flex items-center gap-1.5">
              <Filter className="h-3 w-3" />
              Filters
              {[since, until, keyFilter].filter(Boolean).length > 0 && (
                <span className="ml-1 inline-flex items-center justify-center w-4 h-4 rounded-full bg-primary/20 text-primary text-[10px] font-bold">
                  {[since, until, keyFilter].filter(Boolean).length}
                </span>
              )}
            </span>
            {(since || until || keyFilter) && (
              <Button
                variant="ghost"
                size="sm"
                icon={<RotateCcw className="h-3.5 w-3.5" />}
                onClick={() => { setSince(''); setUntil(''); setKeyFilter(''); setPage(1); }}
              >
                Clear
              </Button>
            )}
          </div>
          <div className="p-4 flex flex-wrap items-end gap-3">
            <div>
              <label className="block text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-1.5">
                <Clock className="h-3 w-3 inline mr-1" />
                From
              </label>
              <input type="date" value={since} onChange={(e) => { setSince(e.target.value); setPage(1); }} className={inputStyle} />
            </div>
            <div>
              <label className="block text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-1.5">
                <Clock className="h-3 w-3 inline mr-1" />
                To
              </label>
              <input type="date" value={until} onChange={(e) => { setUntil(e.target.value); setPage(1); }} className={inputStyle} />
            </div>
            <div>
              <label className="block text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-1.5">
                API Key
              </label>
              <select value={keyFilter} onChange={(e) => { setKeyFilter(e.target.value); setPage(1); }} className={selectStyle}>
                <option value="">All API Keys</option>
                {keys?.items?.map((k) => (<option key={k.id} value={k.id}>{k.name}</option>))}
              </select>
            </div>
          </div>
        </div>
      </motion.div>

      {/* Stat Cards */}
      {!summaryLoading && (
        <div className="grid grid-cols-2 md:grid-cols-3 xl:grid-cols-6 gap-3 mb-6">
          <MetricCard
            label="Total Cost"
            value={`$${grandTotals.cost.toFixed(4)}`}
            icon={<DollarSign className="h-4 w-4 text-emerald-400" />}
            index={0}
            reducedMotion={reducedMotion}
          />
          <MetricCard
            label="Requests"
            value={grandTotals.requests.toLocaleString()}
            icon={<MessageSquare className="h-4 w-4 text-blue-400" />}
            index={1}
            reducedMotion={reducedMotion}
          />
          <MetricCard
            label="Input Tokens"
            value={grandTotals.inputTokens.toLocaleString()}
            icon={<ArrowDownToLine className="h-4 w-4 text-violet-400" />}
            index={2}
            reducedMotion={reducedMotion}
          />
          <MetricCard
            label="Cache Read"
            value={grandTotals.cacheReadTokens.toLocaleString()}
            icon={<Zap className="h-4 w-4 text-amber-400" />}
            index={3}
            reducedMotion={reducedMotion}
          />
          <MetricCard
            label="Cache Created"
            value={grandTotals.cacheCreationTokens.toLocaleString()}
            icon={<Zap className="h-4 w-4 text-orange-400" />}
            index={4}
            reducedMotion={reducedMotion}
          />
          <MetricCard
            label="Output Tokens"
            value={grandTotals.outputTokens.toLocaleString()}
            icon={<ArrowUpFromLine className="h-4 w-4 text-rose-400" />}
            index={5}
            reducedMotion={reducedMotion}
          />
        </div>
      )}

      {/* Chart */}
      {!summaryLoading && chartData.length > 0 && (
        <motion.div
          initial={reducedMotion ? false : { opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={reducedMotion ? { duration: 0 } : { duration: 0.35, delay: 0.15, ease: EASE }}
          className="mb-6 rounded-2xl border border-base-300/40 bg-base-100 p-4 sm:p-5 overflow-hidden"
        >
          <h2 className="text-sm font-bold text-base-content/70 mb-4">Token Usage by Model</h2>
          <ResponsiveContainer width="100%" height={300}>
            <BarChart data={chartData}>
              <CartesianGrid strokeDasharray="3 3" stroke="var(--color-base-300)" opacity={0.4} />
              <XAxis dataKey="model" tick={{ fill: 'var(--color-base-content)', fontSize: 12, opacity: 0.5 }} />
              <YAxis tick={{ fill: 'var(--color-base-content)', fontSize: 12, opacity: 0.5 }} />
              <Tooltip
                contentStyle={{ background: 'var(--color-base-200)', border: '1px solid var(--color-base-300)', borderRadius: 12, fontSize: 13 }}
                formatter={(value: number, name: string) => {
                  if (name === 'cost') return [`$${value.toFixed(4)}`, 'Cost'];
                  if (name === 'requests') return [value.toLocaleString(), 'Requests'];
                  return [value.toLocaleString(), name.charAt(0).toUpperCase() + name.slice(1)];
                }}
              />
              <Legend wrapperStyle={{ fontSize: 12, opacity: 0.6 }} />
              <Bar dataKey="input" stackId="a" fill="var(--color-primary)" name="Input" />
              <Bar dataKey="cache" stackId="a" fill="#60a5fa" name="Cache Read" />
              <Bar dataKey="cacheCreation" stackId="a" fill="#fb923c" name="Cache Created" />
              <Bar dataKey="output" stackId="a" fill="var(--color-secondary)" name="Output" radius={[4, 4, 0, 0]} />
            </BarChart>
          </ResponsiveContainer>
        </motion.div>
      )}

      {/* Table */}
      <motion.div
        initial={reducedMotion ? false : { opacity: 0, y: 12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={reducedMotion ? { duration: 0 } : { duration: 0.35, delay: 0.2, ease: EASE }}
      >
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
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">Time</th>
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">Key</th>
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">Model</th>
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">Protocol</th>
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45 text-right">Input</th>
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45 text-right">Cache Read</th>
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45 text-right">Cache Created</th>
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45 text-right">Output</th>
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45 text-right">Cost</th>
                  </tr>
                </thead>
                <tbody>
                  {usageItems.map((item) => (
                    <tr key={item.id} className="border-b border-base-200/40 hover:bg-base-200/20 transition-colors">
                      <td className="font-mono text-[13px] text-base-content/55">
                        {new Date(item.created_at).toLocaleString()}
                      </td>
                      <td className="font-mono text-sm text-base-content/55">{item.key_id.substring(0, 8)}...</td>
                      <td className="font-mono text-sm font-medium">{item.model_name}</td>
                      <td><Badge variant={item.protocol === 'openai' ? 'blue' : 'purple'}>{item.protocol}</Badge></td>
                      <td className="font-mono text-sm text-right text-base-content/55">{(item.input_tokens ?? 0).toLocaleString()}</td>
                      <td className="font-mono text-sm text-right text-base-content/55">{(item.cache_read_tokens ?? 0).toLocaleString()}</td>
                      <td className="font-mono text-sm text-right text-base-content/55">{(item.cache_creation_tokens ?? 0).toLocaleString()}</td>
                      <td className="font-mono text-sm text-right text-base-content/55">{(item.output_tokens ?? 0).toLocaleString()}</td>
                      <td className="font-mono text-sm text-right">${item.cost.toFixed(6)}</td>
                    </tr>
                  ))}
                  {usageItems.length === 0 && (
                    <tr>
                      <td colSpan={9} className="text-center py-12 text-base-content/40 text-sm">
                        No usage data for the selected period
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>

            {totalPages > 1 && (
              <div className="mt-4 flex items-center justify-between text-sm">
                <span className="text-xs text-base-content/40">Total {data?.total ?? 0}</span>
                <div className="join">
                  <Button variant="ghost" size="sm" disabled={page <= 1} onClick={() => setPage(page - 1)}>
                    Previous
                  </Button>
                  <span className="px-3 flex items-center text-sm text-base-content/60">
                    {page} / {totalPages}
                  </span>
                  <Button variant="ghost" size="sm" disabled={page >= totalPages} onClick={() => setPage(page + 1)}>
                    Next
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
