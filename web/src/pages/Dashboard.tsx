import { MessageSquare, DollarSign, Zap, TrendingUp, Activity, Clock } from 'lucide-react';
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

  return (
    <div>
      {/* Page Header */}
      <div className="mb-8">
        <h1 className="text-2xl font-bold">Dashboard</h1>
        <p className="text-sm text-base-content/55 mt-1">Real-time overview of your LLM gateway activity</p>
      </div>

      {/* Metrics Grid */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-3 mb-8">
        {/* Today's Requests */}
        <div className="stat-card rounded-xl border border-base-300/50 bg-base-100/60 p-4">
          <div className="flex items-center justify-between mb-3">
            <span className="text-[11px] font-semibold uppercase tracking-wider text-base-content/50">Today's Requests</span>
            <MessageSquare className="h-4 w-4 text-base-content/35" />
          </div>
          <div className="font-mono text-2xl font-bold">{todayRequests.toLocaleString()}</div>
        </div>

        {/* Today's Cost */}
        <div className="stat-card rounded-xl border border-base-300/50 bg-base-100/60 p-4">
          <div className="flex items-center justify-between mb-3">
            <span className="text-[11px] font-semibold uppercase tracking-wider text-base-content/50">Today's Cost</span>
            <DollarSign className="h-4 w-4 text-base-content/35" />
          </div>
          <div className="font-mono text-2xl font-bold">${todayCost.toFixed(4)}</div>
        </div>

        {/* Monthly Cost */}
        <div className="stat-card rounded-xl border border-base-300/50 bg-base-100/60 p-4">
          <div className="flex items-center justify-between mb-3">
            <span className="text-[11px] font-semibold uppercase tracking-wider text-base-content/50">Monthly Cost</span>
            <TrendingUp className="h-4 w-4 text-base-content/35" />
          </div>
          <div className="font-mono text-2xl font-bold">${monthCost.toFixed(2)}</div>
        </div>

        {/* Active Models */}
        <div className="stat-card rounded-xl border border-base-300/50 bg-base-100/60 p-4">
          <div className="flex items-center justify-between mb-3">
            <span className="text-[11px] font-semibold uppercase tracking-wider text-base-content/50">Active Models</span>
            <Zap className="h-4 w-4 text-base-content/35" />
          </div>
          <div className="font-mono text-2xl font-bold">{totalModels}</div>
        </div>
      </div>

      {/* Live Metrics Bar */}
      <div className="flex flex-wrap gap-3 mb-8">
        <div className="flex items-center gap-2.5 rounded-lg border border-base-300/50 bg-base-100/60 px-4 py-2.5">
          <Activity className="h-4 w-4 text-primary/70" />
          <span className="text-[11px] uppercase tracking-wider text-base-content/50 font-semibold">Avg Latency</span>
          <span className="font-mono text-sm font-bold">{avgLatency}<span className="text-base-content/45 ml-0.5">ms</span></span>
        </div>
        <div className="flex items-center gap-2.5 rounded-lg border border-base-300/50 bg-base-100/60 px-4 py-2.5">
          <TrendingUp className="h-4 w-4 text-primary/70" />
          <span className="text-[11px] uppercase tracking-wider text-base-content/50 font-semibold">Success Rate</span>
          <span className="font-mono text-sm font-bold">{successRate}<span className="text-base-content/45 ml-0.5">%</span></span>
        </div>
        <div className="flex items-center gap-2.5 rounded-lg border border-base-300/50 bg-base-100/60 px-4 py-2.5">
          <Clock className="h-4 w-4 text-primary/70" />
          <span className="text-[11px] uppercase tracking-wider text-base-content/50 font-semibold">Recent</span>
          <span className="font-mono text-sm font-bold">{recentLogs?.items?.length ?? 0}<span className="text-base-content/45 ml-0.5">reqs</span></span>
        </div>
      </div>

      {/* Recent Requests */}
      <div>
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-sm font-semibold">Recent Requests</h2>
          <span className="text-xs font-mono text-base-content/45">{recentLogs?.items?.length ?? 0}</span>
        </div>

        <div className="rounded-xl border border-base-300/50 bg-base-100/60 overflow-hidden">
          <div className="overflow-x-auto">
            <table className="table table-sm">
              <thead>
                <tr className="border-b border-base-300/50">
                  <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/50">Time</th>
                  <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/50">Model</th>
                  <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/50">Protocol</th>
                  <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/50">Status</th>
                  <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/50">Tokens</th>
                  <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/50">Latency</th>
                </tr>
              </thead>
              <tbody>
                {recentLogs?.items?.map((log) => (
                  <tr key={log.id} className="border-b border-base-200/50 hover:bg-base-200/30 transition-colors">
                    <td className="mono text-[13px] text-base-content/60">
                      {new Date(log.created_at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })}
                    </td>
                    <td className="mono text-[13px] font-medium">{log.model_name}</td>
                    <td><Badge variant={log.protocol === 'openai' ? 'blue' : 'purple'}>{log.protocol}</Badge></td>
                    <td><Badge variant={log.status_code < 400 ? 'green' : log.status_code < 500 ? 'amber' : 'red'}>{log.status_code}</Badge></td>
                    <td className="mono text-[13px] text-base-content/60">
                      {log.input_tokens ?? 0} + {log.output_tokens ?? 0}
                    </td>
                    <td className="mono text-[13px] text-base-content/60">{log.latency_ms}ms</td>
                  </tr>
                ))}
                {(!recentLogs?.items?.length) && (
                  <tr>
                    <td colSpan={6} className="text-center py-16 text-base-content/45 text-sm">
                      No requests yet
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
