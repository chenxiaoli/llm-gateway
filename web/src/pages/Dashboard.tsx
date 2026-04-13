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
        <p className="text-sm text-base-content/50 mt-1">Real-time overview of your LLM gateway activity</p>
      </div>

      {/* Live Metrics */}
      <div className="grid grid-cols-3 gap-4 mb-6">
        <div className="stat bg-base-100 rounded-box p-4 shadow-sm">
          <div className="stat-title text-base-content/50 flex items-center gap-2">
            <Activity className="h-4 w-4 text-primary" /> Avg Latency
          </div>
          <div className="stat-value text-2xl font-mono">{avgLatency}<span className="text-sm text-base-content/40 ml-1">ms</span></div>
        </div>
        <div className="stat bg-base-100 rounded-box p-4 shadow-sm">
          <div className="stat-title text-base-content/50 flex items-center gap-2">
            <TrendingUp className="h-4 w-4 text-primary" /> Success Rate
          </div>
          <div className="stat-value text-2xl font-mono">{successRate}<span className="text-sm text-base-content/40 ml-1">%</span></div>
        </div>
        <div className="stat bg-base-100 rounded-box p-4 shadow-sm">
          <div className="stat-title text-base-content/50 flex items-center gap-2">
            <Clock className="h-4 w-4 text-primary" /> Recent
          </div>
          <div className="stat-value text-2xl font-mono">{recentLogs?.items?.length ?? 0}<span className="text-sm text-base-content/40 ml-1">reqs</span></div>
        </div>
      </div>

      {/* Stat Cards */}
      <div className="grid grid-cols-4 gap-4 mb-8 max-lg:grid-cols-2 max-sm:grid-cols-1">
        <div className="stat-card bg-base-100 rounded-box p-5 shadow-sm animate-fade-in-up">
          <div className="flex items-center justify-between mb-2">
            <span className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Today's Requests</span>
            <MessageSquare className="h-4 w-4 text-primary/60" />
          </div>
          <div className="font-mono text-3xl font-bold">{todayRequests.toLocaleString()}</div>
        </div>
        <div className="stat-card bg-base-100 rounded-box p-5 shadow-sm animate-fade-in-up" style={{ animationDelay: '80ms' }}>
          <div className="flex items-center justify-between mb-2">
            <span className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Today's Cost</span>
            <DollarSign className="h-4 w-4 text-warning/60" />
          </div>
          <div className="font-mono text-3xl font-bold">${todayCost.toFixed(4)}</div>
        </div>
        <div className="stat-card bg-base-100 rounded-box p-5 shadow-sm animate-fade-in-up" style={{ animationDelay: '160ms' }}>
          <div className="flex items-center justify-between mb-2">
            <span className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Monthly Cost</span>
            <TrendingUp className="h-4 w-4 text-info/60" />
          </div>
          <div className="font-mono text-3xl font-bold">${monthCost.toFixed(2)}</div>
        </div>
        <div className="stat-card bg-base-100 rounded-box p-5 shadow-sm animate-fade-in-up" style={{ animationDelay: '240ms' }}>
          <div className="flex items-center justify-between mb-2">
            <span className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Active Models</span>
            <Zap className="h-4 w-4 text-secondary/60" />
          </div>
          <div className="font-mono text-3xl font-bold">{totalModels}</div>
        </div>
      </div>

      {/* Recent Requests */}
      <div className="animate-fade-in-up" style={{ animationDelay: '300ms' }}>
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-base font-semibold">Recent Requests</h2>
          <span className="text-xs font-mono text-base-content/40">({recentLogs?.items?.length ?? 0})</span>
        </div>

        <div className="overflow-x-auto bg-base-100 rounded-box shadow-sm">
          <table className="table table-sm">
            <thead>
              <tr className="border-b border-base-300">
                <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Time</th>
                <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Model</th>
                <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Protocol</th>
                <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Status</th>
                <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Tokens</th>
                <th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Latency</th>
              </tr>
            </thead>
            <tbody>
              {recentLogs?.items?.map((log) => (
                <tr key={log.id} className="border-b border-base-200 hover">
                  <td className="mono text-[13px]">
                    {new Date(log.created_at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })}
                  </td>
                  <td className="mono text-[13px] font-medium">{log.model_name}</td>
                  <td><Badge variant={log.protocol === 'openai' ? 'blue' : 'purple'}>{log.protocol}</Badge></td>
                  <td><Badge variant={log.status_code < 400 ? 'green' : log.status_code < 500 ? 'amber' : 'red'}>{log.status_code}</Badge></td>
                  <td className="mono text-[13px]">
                    {log.input_tokens ?? 0} + {log.output_tokens ?? 0}
                  </td>
                  <td className="mono text-[13px]">{log.latency_ms}ms</td>
                </tr>
              ))}
              {(!recentLogs?.items?.length) && (
                <tr>
                  <td colSpan={6} className="text-center py-12 text-base-content/40">
                    No requests yet
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
