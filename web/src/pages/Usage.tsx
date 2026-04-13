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

  const byModel: Record<string, { model: string; requests: number; cost: number }> = {};
  usageItems.forEach((r) => {
    if (!byModel[r.model_name]) {
      byModel[r.model_name] = { model: r.model_name, requests: 0, cost: 0 };
    }
    byModel[r.model_name].requests += 1;
    byModel[r.model_name].cost += r.cost;
  });
  const chartData = Object.values(byModel).sort((a, b) => b.cost - a.cost);

  const totalPages = Math.ceil((data?.total ?? 0) / pageSize);

  const stats = [
    { label: 'Total Cost', value: `$${totalCost.toFixed(4)}`, icon: DollarSign, accent: '#06d6a0' },
    { label: 'Total Requests', value: totalRequests.toLocaleString(), icon: MessageSquare, accent: '#3b82f6' },
    { label: 'Input Tokens', value: totalInputTokens.toLocaleString(), accent: '#f59e0b' },
    { label: 'Output Tokens', value: totalOutputTokens.toLocaleString(), accent: '#a855f7' },
  ];

  return (
    <div>
      <div className="mb-6">
        <h1 className="font-display text-2xl font-bold text-[#ededed]">Usage</h1>
      </div>

      {/* Filters */}
      <div className="mb-4 flex flex-wrap items-center gap-3">
        <input
          type="date"
          value={since}
          onChange={(e) => setSince(e.target.value)}
          className="h-9 rounded-lg border border-[#262626] bg-[#141414] px-3 text-sm text-[#ededed] outline-none focus:border-accent/50 transition-colors"
        />
        <input
          type="date"
          value={until}
          onChange={(e) => setUntil(e.target.value)}
          className="h-9 rounded-lg border border-[#262626] bg-[#141414] px-3 text-sm text-[#ededed] outline-none focus:border-accent/50 transition-colors"
        />
        <select
          value={keyFilter}
          onChange={(e) => setKeyFilter(e.target.value)}
          className="h-9 rounded-lg border border-[#262626] bg-[#141414] px-3 text-sm text-[#ededed] outline-none focus:border-accent/50 transition-colors"
        >
          <option value="">All API Keys</option>
          {keys?.items?.map((k) => (
            <option key={k.id} value={k.id}>{k.name}</option>
          ))}
        </select>
      </div>

      {/* Stat cards */}
      <div className="grid grid-cols-4 gap-4 mb-6 max-lg:grid-cols-2 max-sm:grid-cols-1">
        {stats.map((s, i) => {
          const Icon = s.icon;
          return (
            <div
              key={s.label}
              className="relative overflow-hidden rounded-xl border border-[#1e1e1e] bg-[#0a0a0a] p-5 pl-6 animate-fade-in-up"
              style={{ animationDelay: `${i * 60}ms` }}
            >
              <div className="absolute top-0 left-0 right-0 h-[2px] opacity-40" style={{ background: `linear-gradient(90deg, ${s.accent}, transparent)` }} />
              <div className="text-[11px] font-semibold uppercase tracking-[0.06em] text-[#555555] mb-2">{s.label}</div>
              <div className="font-mono text-[28px] font-bold text-[#ededed] leading-tight">{s.value}</div>
              {Icon && <Icon className="absolute top-[18px] right-[18px] h-[18px] w-[18px] text-[#555555] opacity-40" />}
            </div>
          );
        })}
      </div>

      {/* Chart */}
      {chartData.length > 0 && (
        <div className="mb-6 rounded-xl border border-[#1e1e1e] bg-[#0a0a0a] p-5">
          <h2 className="font-display text-base font-semibold text-[#ededed] mb-4">Cost by Model</h2>
          <ResponsiveContainer width="100%" height={300}>
            <BarChart data={chartData}>
              <CartesianGrid strokeDasharray="3 3" stroke="#27272a" />
              <XAxis dataKey="model" tick={{ fill: '#71717a', fontSize: 12 }} />
              <YAxis tick={{ fill: '#71717a', fontSize: 12 }} />
              <Tooltip
                contentStyle={{ background: '#1e1e22', border: '1px solid #3f3f46', borderRadius: 8 }}
                labelStyle={{ color: '#fafafa' }}
                formatter={(value: number) => [`$${value.toFixed(4)}`, 'Cost']}
              />
              <Bar dataKey="cost" fill="#06d6a0" name="Cost ($)" radius={[4, 4, 0, 0]} />
            </BarChart>
          </ResponsiveContainer>
        </div>
      )}

      {/* Table */}
      {isLoading ? (
        <div className="flex items-center justify-center py-12 text-[#555555]">Loading...</div>
      ) : (
        <>
          <div className="overflow-x-auto rounded-xl border border-[#1e1e1e]">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-[#1e1e1e]">
                  <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Time</th>
                  <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Key ID</th>
                  <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Model</th>
                  <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Protocol</th>
                  <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Input Tokens</th>
                  <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Output Tokens</th>
                  <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Cost</th>
                </tr>
              </thead>
              <tbody>
                {usageItems.map((item) => (
                  <tr key={item.id} className="border-b border-[#1e1e1e]/50 hover:bg-white/[0.02] transition-colors">
                    <td className="px-4 py-2.5"><span className="mono text-[13px]">{new Date(item.created_at).toLocaleString()}</span></td>
                    <td className="px-4 py-2.5"><span className="mono">{item.key_id.substring(0, 8)}...</span></td>
                    <td className="px-4 py-2.5"><span className="mono">{item.model_name}</span></td>
                    <td className="px-4 py-2.5"><Badge variant={item.protocol === 'openai' ? 'blue' : 'purple'}>{item.protocol}</Badge></td>
                    <td className="px-4 py-2.5"><span className="mono">{item.input_tokens ?? '-'}</span></td>
                    <td className="px-4 py-2.5"><span className="mono">{item.output_tokens ?? '-'}</span></td>
                    <td className="px-4 py-2.5"><span className="mono">${item.cost.toFixed(6)}</span></td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          {totalPages > 1 && (
            <div className="mt-4 flex items-center justify-between text-sm">
              <span className="text-[#555555]">Total {data?.total ?? 0}</span>
              <div className="flex gap-2">
                <Button variant="ghost" size="sm" disabled={page <= 1} onClick={() => setPage(page - 1)}>Previous</Button>
                <span className="px-2 text-[#888888]">{page} / {totalPages}</span>
                <Button variant="ghost" size="sm" disabled={page >= totalPages} onClick={() => setPage(page + 1)}>Next</Button>
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
}
