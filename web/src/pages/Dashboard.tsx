import { MessageSquare, DollarSign, BarChart3, Zap } from 'lucide-react';
import { useLogs } from '../hooks/useLogs';
import { useUsage } from '../hooks/useUsage';
import { Badge } from '../components/ui/Badge';

function startOfDay() {
  const d = new Date();
  d.setHours(0, 0, 0, 0);
  return d.toISOString();
}

function startOfMonth() {
  const d = new Date();
  d.setDate(1);
  d.setHours(0, 0, 0, 0);
  return d.toISOString();
}

export default function Dashboard() {
  const { data: allUsage } = useUsage({}, 1, 99999);
  const { data: todayUsage } = useUsage({ since: startOfDay() }, 1, 99999);
  const { data: monthUsage } = useUsage({ since: startOfMonth() }, 1, 99999);
  const { data: recentLogs } = useLogs({});

  const todayItems = todayUsage?.items ?? [];
  const monthItems = monthUsage?.items ?? [];
  const allItems = allUsage?.items ?? [];

  const todayRequests = todayItems.length;
  const todayCost = todayItems.reduce((sum, r) => sum + r.cost, 0);
  const monthCost = monthItems.reduce((sum, r) => sum + r.cost, 0);
  const totalModels = new Set(allItems.map(r => r.model_name)).size;

  const stats = [
    { label: "Today's Requests", value: todayRequests.toLocaleString(), icon: MessageSquare, accent: '#06d6a0' },
    { label: "Today's Cost", value: `$${todayCost.toFixed(4)}`, icon: DollarSign, accent: '#f59e0b' },
    { label: 'Monthly Cost', value: `$${monthCost.toFixed(2)}`, icon: DollarSign, accent: '#3b82f6' },
    { label: 'Active Models', value: String(totalModels), icon: Zap, accent: '#a855f7' },
  ];

  return (
    <div>
      <div className="mb-6">
        <h1 className="font-display text-2xl font-bold text-[#ededed]">Dashboard</h1>
      </div>

      <div className="grid grid-cols-4 gap-4 mb-6 max-lg:grid-cols-2 max-sm:grid-cols-1">
        {stats.map((s, i) => {
          const Icon = s.icon;
          return (
            <div
              key={s.label}
              className="relative overflow-hidden rounded-xl border border-[#1e1e1e] bg-[#0a0a0a] p-5 pl-6 transition-all duration-250 hover:border-[#262626] hover:-translate-y-0.5 hover:shadow-lg hover:shadow-black/20 animate-fade-in-up"
              style={{ animationDelay: `${i * 60}ms` }}
            >
              <div className="absolute top-0 left-0 right-0 h-[2px] bg-gradient-to-r opacity-0 hover:opacity-100 transition-opacity" style={{ background: `linear-gradient(90deg, ${s.accent}, transparent)` }} />
              <div className="text-[11px] font-semibold uppercase tracking-[0.06em] text-[#555555] mb-2">{s.label}</div>
              <div className="font-mono text-[28px] font-bold text-[#ededed] leading-tight">{s.value}</div>
              <Icon className="absolute top-[18px] right-[18px] h-[18px] w-[18px] text-[#555555] opacity-40" />
            </div>
          );
        })}
      </div>

      <div className="flex items-center justify-between mb-4">
        <h2 className="font-display text-base font-semibold text-[#ededed]">Recent Requests</h2>
      </div>

      <div className="overflow-x-auto rounded-xl border border-[#1e1e1e]">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-[#1e1e1e]">
              <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Time</th>
              <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Model</th>
              <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Protocol</th>
              <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Status</th>
              <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Tokens</th>
              <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Cost</th>
              <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Latency</th>
            </tr>
          </thead>
          <tbody>
            {recentLogs?.items?.map((log) => (
              <tr key={log.id} className="border-b border-[#1e1e1e]/50 hover:bg-white/[0.02] transition-colors">
                <td className="px-4 py-2.5"><span className="mono text-[13px]">{new Date(log.created_at).toLocaleString()}</span></td>
                <td className="px-4 py-2.5"><span className="mono">{log.model_name}</span></td>
                <td className="px-4 py-2.5"><Badge variant={log.protocol === 'openai' ? 'blue' : 'purple'}>{log.protocol}</Badge></td>
                <td className="px-4 py-2.5"><Badge variant={log.status_code < 400 ? 'green' : log.status_code < 500 ? 'amber' : 'red'}>{log.status_code}</Badge></td>
                <td className="px-4 py-2.5"><span className="mono">{log.input_tokens ?? 0} + {log.output_tokens ?? 0}</span></td>
                <td className="px-4 py-2.5"><span className="mono">${log.cost.toFixed(6)}</span></td>
                <td className="px-4 py-2.5"><span className="mono">{log.latency_ms}ms</span></td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
