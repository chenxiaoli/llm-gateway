import { Activity, MessageSquare, DollarSign, Zap, TrendingUp, Clock } from 'lucide-react';
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
  const avgLatency = recentLogs?.items?.length
    ? Math.round(recentLogs.items.reduce((sum, r) => sum + r.latency_ms, 0) / recentLogs.items.length)
    : 0;

  const successRate = recentLogs?.items?.length
    ? Math.round((recentLogs.items.filter(r => r.status_code < 400).length / recentLogs.items.length) * 100)
    : 100;

  const stats = [
    { label: "Today's Requests", value: todayRequests.toLocaleString(), icon: MessageSquare, color: '#06d6a0', bg: 'rgba(6, 214, 160, 0.08)', border: 'rgba(6, 214, 160, 0.15)' },
    { label: "Today's Cost", value: `$${todayCost.toFixed(4)}`, icon: DollarSign, color: '#f59e0b', bg: 'rgba(245, 158, 11, 0.08)', border: 'rgba(245, 158, 11, 0.15)' },
    { label: 'Monthly Cost', value: `$${monthCost.toFixed(2)}`, icon: TrendingUp, color: '#3b82f6', bg: 'rgba(59, 130, 246, 0.08)', border: 'rgba(59, 130, 246, 0.15)' },
    { label: 'Active Models', value: String(totalModels), icon: Zap, color: '#a855f7', bg: 'rgba(168, 85, 247, 0.08)', border: 'rgba(168, 85, 247, 0.15)' },
  ];

  return (
    <div className="ambient-glow">
      {/* Page Header */}
      <div className="mb-8">
        <div className="flex items-center gap-3 mb-1">
          <h1 className="font-display text-2xl font-bold text-[#ededed] tracking-tight">Dashboard</h1>
          <div className="flex items-center gap-1.5 rounded-full px-2.5 py-1 text-[11px] font-medium" style={{ background: 'rgba(6, 214, 160, 0.08)', color: '#06d6a0', border: '1px solid rgba(6, 214, 160, 0.15)' }}>
            <div className="pulse-dot h-1.5 w-1.5 rounded-full bg-accent" />
            System Online
          </div>
        </div>
        <p className="text-sm text-[#777777]">Real-time overview of your LLM gateway activity</p>
      </div>

      {/* Live Metrics Bar */}
      <div className="mb-6 grid grid-cols-3 gap-3">
        <div className="flex items-center gap-3 rounded-xl p-4" style={{ background: '#0a0a0a', border: '1px solid rgba(30, 30, 30, 0.8)' }}>
          <div className="flex h-9 w-9 items-center justify-center rounded-lg" style={{ background: 'rgba(6, 214, 160, 0.1)' }}>
            <Activity className="h-4 w-4" style={{ color: '#06d6a0' }} />
          </div>
          <div>
            <div className="text-[11px] font-medium text-[#777777] uppercase tracking-wider">Avg Latency</div>
            <div className="font-mono text-lg font-bold text-[#ededed]">{avgLatency}<span className="text-xs text-[#777777] ml-1">ms</span></div>
          </div>
        </div>
        <div className="flex items-center gap-3 rounded-xl p-4" style={{ background: '#0a0a0a', border: '1px solid rgba(30, 30, 30, 0.8)' }}>
          <div className="flex h-9 w-9 items-center justify-center rounded-lg" style={{ background: 'rgba(6, 214, 160, 0.1)' }}>
            <TrendingUp className="h-4 w-4" style={{ color: '#06d6a0' }} />
          </div>
          <div>
            <div className="text-[11px] font-medium text-[#777777] uppercase tracking-wider">Success Rate</div>
            <div className="font-mono text-lg font-bold text-[#ededed]">{successRate}<span className="text-xs text-[#777777] ml-1">%</span></div>
          </div>
        </div>
        <div className="flex items-center gap-3 rounded-xl p-4" style={{ background: '#0a0a0a', border: '1px solid rgba(30, 30, 30, 0.8)' }}>
          <div className="flex h-9 w-9 items-center justify-center rounded-lg" style={{ background: 'rgba(6, 214, 160, 0.1)' }}>
            <Clock className="h-4 w-4" style={{ color: '#06d6a0' }} />
          </div>
          <div>
            <div className="text-[11px] font-medium text-[#777777] uppercase tracking-wider">Recent</div>
            <div className="font-mono text-lg font-bold text-[#ededed]">{recentLogs?.items?.length ?? 0}<span className="text-xs text-[#777777] ml-1">reqs</span></div>
          </div>
        </div>
      </div>

      {/* Stat Cards */}
      <div className="grid grid-cols-4 gap-4 mb-8 max-lg:grid-cols-2 max-sm:grid-cols-1">
        {stats.map((s, i) => {
          const Icon = s.icon;
          return (
            <div
              key={s.label}
              className="stat-card rounded-xl p-5 animate-fade-in-up"
              style={{
                animationDelay: `${i * 80}ms`,
                background: s.bg,
                border: `1px solid ${s.border}`,
              }}
            >
              <div className="flex items-center justify-between mb-3">
                <span className="text-[11px] font-semibold uppercase tracking-[0.08em] text-[#888888]">{s.label}</span>
                <div className="flex h-7 w-7 items-center justify-center rounded-md" style={{ background: `${s.color}15` }}>
                  <Icon className="h-3.5 w-3.5" style={{ color: s.color, opacity: 0.8 }} />
                </div>
              </div>
              <div className="font-mono text-[28px] font-bold text-[#ededed] leading-tight animate-count-up" style={{ animationDelay: `${i * 80 + 200}ms` }}>
                {s.value}
              </div>
            </div>
          );
        })}
      </div>

      {/* Recent Requests */}
      <div className="animate-fade-in-up" style={{ animationDelay: '350ms' }}>
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2">
            <h2 className="font-display text-base font-semibold text-[#ededed]">Recent Requests</h2>
            <span className="text-[11px] font-mono text-[#666666]">({recentLogs?.items?.length ?? 0})</span>
          </div>
        </div>

        <div className="rounded-xl overflow-hidden" style={{ border: '1px solid rgba(30, 30, 30, 0.8)' }}>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr style={{ background: 'rgba(17, 17, 17, 0.8)', borderBottom: '1px solid rgba(30, 30, 30, 0.8)' }}>
                  <th className="px-4 py-3 text-left text-[10px] font-semibold uppercase tracking-[0.08em] text-[#777777]">Time</th>
                  <th className="px-4 py-3 text-left text-[10px] font-semibold uppercase tracking-[0.08em] text-[#777777]">Model</th>
                  <th className="px-4 py-3 text-left text-[10px] font-semibold uppercase tracking-[0.08em] text-[#777777]">Protocol</th>
                  <th className="px-4 py-3 text-left text-[10px] font-semibold uppercase tracking-[0.08em] text-[#777777]">Status</th>
                  <th className="px-4 py-3 text-left text-[10px] font-semibold uppercase tracking-[0.08em] text-[#777777]">Tokens</th>
                  <th className="px-4 py-3 text-left text-[10px] font-semibold uppercase tracking-[0.08em] text-[#777777]">Latency</th>
                </tr>
              </thead>
              <tbody>
                {recentLogs?.items?.map((log, i) => (
                  <tr
                    key={log.id}
                    className="transition-colors duration-150"
                    style={{
                      borderBottom: '1px solid rgba(30, 30, 30, 0.4)',
                      animation: `slideInRight 0.3s ease-out ${i * 30}ms both`,
                    }}
                    onMouseEnter={(e) => {
                      (e.currentTarget as HTMLElement).style.background = 'rgba(255, 255, 255, 0.015)';
                    }}
                    onMouseLeave={(e) => {
                      (e.currentTarget as HTMLElement).style.background = 'transparent';
                    }}
                  >
                    <td className="px-4 py-3">
                      <span className="mono text-[13px] text-[#888888]">
                        {new Date(log.created_at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })}
                      </span>
                    </td>
                    <td className="px-4 py-3">
                      <span className="mono text-[13px] text-[#ededed] font-medium">{log.model_name}</span>
                    </td>
                    <td className="px-4 py-3">
                      <Badge variant={log.protocol === 'openai' ? 'blue' : 'purple'}>{log.protocol}</Badge>
                    </td>
                    <td className="px-4 py-3">
                      <Badge variant={log.status_code < 400 ? 'green' : log.status_code < 500 ? 'amber' : 'red'}>
                        {log.status_code}
                      </Badge>
                    </td>
                    <td className="px-4 py-3">
                      <span className="mono text-[13px] text-[#888888]">
                        <span className="text-[#ededed]">{log.input_tokens ?? 0}</span>
                        <span className="text-[#666666] mx-1">+</span>
                        <span className="text-[#ededed]">{log.output_tokens ?? 0}</span>
                      </span>
                    </td>
                    <td className="px-4 py-3">
                      <span className={cn(
                        'mono text-[13px]',
                        log.latency_ms > 5000 ? 'text-[#f59e0b]' : log.latency_ms > 10000 ? 'text-[#ef4444]' : 'text-[#888888]',
                      )}>
                        {log.latency_ms}ms
                      </span>
                    </td>
                  </tr>
                ))}
                {(!recentLogs?.items?.length) && (
                  <tr>
                    <td colSpan={6} className="px-4 py-16 text-center">
                      <div className="text-[#555555] text-sm">No requests yet</div>
                      <div className="text-[#444444] text-xs mt-1">Requests will appear here as they come in</div>
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>
  );
}

function cn(...classes: (string | undefined | false)[]) {
  return classes.filter(Boolean).join(' ');
}
