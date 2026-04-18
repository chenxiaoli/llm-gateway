import { useState } from 'react';
import { DollarSign, MessageSquare } from 'lucide-react';
import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from 'recharts';
import { useUsage } from '../hooks/useUsage';
import { useKeys } from '../hooks/useKeys';
import { Button } from '../components/ui/Button';
import { Badge } from '../components/ui/Badge';

export default function Usage() {
  const [since, setSince] = useState('');
  const [until, setUntil] = useState('');
  const [keyFilter, setKeyFilter] = useState('');
  const [page, setPage] = useState(1);
  const [pageSize] = useState(20);

  const { data, isLoading } = useUsage(
    { since: since || undefined, until: until || undefined, key_id: keyFilter || undefined },
    page,
    pageSize,
  );
  const { data: keys } = useKeys();

  const usageItems = data?.items ?? [];
  const totalCost = usageItems.reduce((sum, r) => sum + r.cost, 0);
  const totalRequests = usageItems.length;
  const totalInputTokens = usageItems.reduce((sum, r) => sum + (r.input_tokens ?? 0), 0);
  const totalOutputTokens = usageItems.reduce((sum, r) => sum + (r.output_tokens ?? 0), 0);
  const totalCacheReadTokens = usageItems.reduce((sum, r) => sum + (r.cache_read_tokens ?? 0), 0);

  const byModel: Record<string, { model: string; requests: number; cost: number }> = {};
  usageItems.forEach((r) => {
    if (!byModel[r.model_name]) { byModel[r.model_name] = { model: r.model_name, requests: 0, cost: 0 }; }
    byModel[r.model_name].requests += 1;
    byModel[r.model_name].cost += r.cost;
  });
  const chartData = Object.values(byModel).sort((a, b) => b.cost - a.cost);
  const totalPages = Math.ceil((data?.total ?? 0) / pageSize);

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

      {/* Stat Cards */}
      <div className="grid grid-cols-5 gap-4 mb-6 max-lg:grid-cols-2 max-sm:grid-cols-1">
        <div className="stat bg-base-100 rounded-box p-4 shadow-sm">
          <div className="stat-title text-base-content/50"><DollarSign className="h-4 w-4 inline mr-1" />Total Cost</div>
          <div className="stat-value text-2xl font-mono">${totalCost.toFixed(4)}</div>
        </div>
        <div className="stat bg-base-100 rounded-box p-4 shadow-sm">
          <div className="stat-title text-base-content/50"><MessageSquare className="h-4 w-4 inline mr-1" />Requests</div>
          <div className="stat-value text-2xl font-mono">{totalRequests.toLocaleString()}</div>
        </div>
        <div className="stat bg-base-100 rounded-box p-4 shadow-sm">
          <div className="stat-title text-base-content/50">Input Tokens</div>
          <div className="stat-value text-2xl font-mono">{totalInputTokens.toLocaleString()}</div>
        </div>
        <div className="stat bg-base-100 rounded-box p-4 shadow-sm">
          <div className="stat-title text-base-content/50">Cache Read</div>
          <div className="stat-value text-2xl font-mono">{totalCacheReadTokens.toLocaleString()}</div>
        </div>
        <div className="stat bg-base-100 rounded-box p-4 shadow-sm">
          <div className="stat-title text-base-content/50">Output Tokens</div>
          <div className="stat-value text-2xl font-mono">{totalOutputTokens.toLocaleString()}</div>
        </div>
      </div>

      {/* Chart */}
      {chartData.length > 0 && (
        <div className="mb-6 bg-base-100 rounded-box p-5 shadow-sm">
          <h2 className="text-base font-semibold mb-4">Cost by Model</h2>
          <ResponsiveContainer width="100%" height={300}>
            <BarChart data={chartData}>
              <CartesianGrid strokeDasharray="3 3" stroke="var(--color-base-300)" />
              <XAxis dataKey="model" tick={{ fill: 'var(--color-base-content)', fontSize: 12, opacity: 0.6 }} />
              <YAxis tick={{ fill: 'var(--color-base-content)', fontSize: 12, opacity: 0.6 }} />
              <Tooltip contentStyle={{ background: 'var(--color-base-200)', border: '1px solid var(--color-base-300)', borderRadius: 8 }} />
              <Bar dataKey="cost" fill="var(--color-primary)" name="Cost ($)" radius={[4, 4, 0, 0]} />
            </BarChart>
          </ResponsiveContainer>
        </div>
      )}

      {/* Table */}
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
