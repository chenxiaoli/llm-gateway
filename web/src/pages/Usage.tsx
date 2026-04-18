import { useState } from 'react';
import { DollarSign, MessageSquare } from 'lucide-react';
import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, Legend, ResponsiveContainer } from 'recharts';
import { useUsage, useUsageSummary } from '../hooks/useUsage';
import { useKeys } from '../hooks/useKeys';
import { Button } from '../components/ui/Button';
import { Badge } from '../components/ui/Badge';
import type { UsageSummaryRecord } from '../types';

export default function Usage() {
  const [since, setSince] = useState('');
  const [until, setUntil] = useState('');
  const [keyFilter, setKeyFilter] = useState('');
  const [page, setPage] = useState(1);
  const [pageSize] = useState(20);

  const filter = { since: since || undefined, until: until || undefined, key_id: keyFilter || undefined };

  const { data, isLoading } = useUsage(filter, page, pageSize);
  const { data: summaryData, isLoading: summaryLoading } = useUsageSummary(filter);
  const { data: keys } = useKeys();

  const usageItems = data?.items ?? [];
  const totalPages = Math.ceil((data?.total ?? 0) / pageSize);

  // Aggregate totals from server-side summary (full filter range, not just current page)
  const summary: UsageSummaryRecord[] = summaryData ?? [];
  const grandTotals = summary.reduce<{ cost: number; requests: number; inputTokens: number; cacheReadTokens: number; outputTokens: number }>(
    (acc, r) => ({
      cost: acc.cost + r.total_cost,
      requests: acc.requests + r.request_count,
      inputTokens: acc.inputTokens + r.total_input_tokens,
      cacheReadTokens: acc.cacheReadTokens + r.total_cache_read_tokens,
      outputTokens: acc.outputTokens + r.total_output_tokens,
    }),
    { cost: 0, requests: 0, inputTokens: 0, cacheReadTokens: 0, outputTokens: 0 }
  );

  // Chart data: stacked bars per model (input / cache / output tokens)
  const chartData = summary.map((r) => ({
    model: r.model_name,
    input: r.total_input_tokens,
    cache: r.total_cache_read_tokens,
    output: r.total_output_tokens,
    cost: r.total_cost,
    requests: r.request_count,
  }));

  return (
    <div>
      <div className="mb-6"><h1 className="text-2xl font-bold">Usage</h1></div>

      {/* Filters */}
      <div className="mb-4 flex flex-wrap items-center gap-3">
        <input type="date" value={since} onChange={(e) => setSince(e.target.value)} className="input input-bordered input-sm" />
        <input type="date" value={until} onChange={(e) => setUntil(e.target.value)} className="input input-bordered input-sm" />
        <select value={keyFilter} onChange={(e) => setKeyFilter(e.target.value)} className="select select-bordered select-sm">
          <option value="">All API Keys</option>
          {keys?.items?.map((k) => (<option key={k.id} value={k.id}>{k.name}</option>))}
        </select>
      </div>

      {/* Stat Cards — fed from server-aggregated summary for full filter range */}
      {!summaryLoading && (
        <div className="grid grid-cols-5 gap-4 mb-6 max-lg:grid-cols-2 max-sm:grid-cols-1">
          <div className="stat bg-base-100 rounded-box p-4 shadow-sm">
            <div className="stat-title text-base-content/50"><DollarSign className="h-4 w-4 inline mr-1" />Total Cost</div>
            <div className="stat-value text-2xl font-mono">${grandTotals.cost.toFixed(4)}</div>
          </div>
          <div className="stat bg-base-100 rounded-box p-4 shadow-sm">
            <div className="stat-title text-base-content/50"><MessageSquare className="h-4 w-4 inline mr-1" />Requests</div>
            <div className="stat-value text-2xl font-mono">{grandTotals.requests.toLocaleString()}</div>
          </div>
          <div className="stat bg-base-100 rounded-box p-4 shadow-sm">
            <div className="stat-title text-base-content/50">Input Tokens</div>
            <div className="stat-value text-2xl font-mono">{grandTotals.inputTokens.toLocaleString()}</div>
          </div>
          <div className="stat bg-base-100 rounded-box p-4 shadow-sm">
            <div className="stat-title text-base-content/50">Cache Read</div>
            <div className="stat-value text-2xl font-mono">{grandTotals.cacheReadTokens.toLocaleString()}</div>
          </div>
          <div className="stat bg-base-100 rounded-box p-4 shadow-sm">
            <div className="stat-title text-base-content/50">Output Tokens</div>
            <div className="stat-value text-2xl font-mono">{grandTotals.outputTokens.toLocaleString()}</div>
          </div>
        </div>
      )}

      {/* Chart — stacked bar (input / cache / output tokens) for full filter range */}
      {!summaryLoading && chartData.length > 0 && (
        <div className="mb-6 bg-base-100 rounded-box p-5 shadow-sm">
          <h2 className="text-base font-semibold mb-4">Token Usage by Model</h2>
          <ResponsiveContainer width="100%" height={300}>
            <BarChart data={chartData}>
              <CartesianGrid strokeDasharray="3 3" stroke="var(--color-base-300)" />
              <XAxis dataKey="model" tick={{ fill: 'var(--color-base-content)', fontSize: 12, opacity: 0.6 }} />
              <YAxis tick={{ fill: 'var(--color-base-content)', fontSize: 12, opacity: 0.6 }} />
              <Tooltip
                contentStyle={{ background: 'var(--color-base-200)', border: '1px solid var(--color-base-300)', borderRadius: 8 }}
                formatter={(value: number, name: string) => {
                  if (name === 'cost') return [`$${value.toFixed(4)}`, 'Cost'];
                  if (name === 'requests') return [value.toLocaleString(), 'Requests'];
                  return [value.toLocaleString(), name.charAt(0).toUpperCase() + name.slice(1)];
                }}
              />
              <Legend />
              <Bar dataKey="input" stackId="a" fill="var(--color-primary)" name="Input" radius={[0, 0, 0, 0]} />
              <Bar dataKey="cache" stackId="a" fill="#60a5fa" name="Cache Read" radius={[0, 0, 0, 0]} />
              <Bar dataKey="output" stackId="a" fill="var(--color-secondary)" name="Output" radius={[4, 4, 0, 0]} />
            </BarChart>
          </ResponsiveContainer>
        </div>
      )}

      {/* Table — paginated, unchanged */}
      {isLoading ? (
        <div className="flex items-center justify-center py-12"><span className="loading loading-spinner loading-lg" /></div>
      ) : (
        <>
          <div className="overflow-x-auto bg-base-100 rounded-box shadow-sm">
            <table className="table table-sm">
              <thead>
                <tr className="border-b border-base-300">
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Time</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Key ID</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Model</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Protocol</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Input Tokens</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Cache Read</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Output Tokens</th>
                  <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Cost</th>
                </tr>
              </thead>
              <tbody>
                {usageItems.map((item) => (
                  <tr key={item.id} className="border-b border-base-200 hover">
                    <td className="mono text-[13px]">{new Date(item.created_at).toLocaleString()}</td>
                    <td className="mono">{item.key_id.substring(0, 8)}...</td>
                    <td className="mono">{item.model_name}</td>
                    <td><Badge variant={item.protocol === 'openai' ? 'blue' : 'purple'}>{item.protocol}</Badge></td>
                    <td className="mono">{item.input_tokens ?? '-'}</td>
                    <td className="mono">{item.cache_read_tokens ?? '-'}</td>
                    <td className="mono">{item.output_tokens ?? '-'}</td>
                    <td className="mono">${item.cost.toFixed(6)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          {totalPages > 1 && (
            <div className="mt-4 flex items-center justify-between text-sm">
              <span className="text-base-content/40">Total {data?.total ?? 0}</span>
              <div className="join">
                <Button variant="ghost" size="sm" disabled={page <= 1} onClick={() => setPage(page - 1)}>Previous</Button>
                <span className="px-3 flex items-center text-base-content/60">{page} / {totalPages}</span>
                <Button variant="ghost" size="sm" disabled={page >= totalPages} onClick={() => setPage(page + 1)}>Next</Button>
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
}
