import { MessageSquare, DollarSign, Zap, TrendingUp, Activity, Server, Network, Users, Cpu, KeyRound, ArrowRight, Clock, AlertTriangle } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { useUsageSummary } from '../hooks/useUsage';
import { useLogs } from '../hooks/useLogs';
import { useProviders } from '../hooks/useProviders';
import { useAllChannels } from '../hooks/useChannels';
import { useAllModels } from '../hooks/useModels';
import { useUsers } from '../hooks/useUsers';
import { useKeys } from '../hooks/useKeys';
import { useSystemInfo } from '../hooks/useSettings';
import { useReducedMotion } from '../hooks/useReducedMotion';
import { Badge } from '../components/ui/Badge';
import { motion } from 'framer-motion';

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

// ── Metric Card ───��──────────────────────────────────────────────────────────
function MetricCard({ label, value, icon, sub, index }: {
  label: string;
  value: string;
  icon: React.ReactNode;
  sub?: string;
  index: number;
}) {
  const reducedMotion = useReducedMotion();
  return (
    <motion.div
      initial={reducedMotion ? false : { opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={reducedMotion ? { duration: 0 } : { duration: 0.35, delay: index * 0.05, ease: [0.16, 1, 0.3, 1] }}
      className="relative rounded-2xl border border-base-300/40 bg-base-100 p-5 overflow-hidden"
    >
      <div className="flex items-center justify-between mb-4">
        <span className="text-xs font-semibold uppercase tracking-wider text-base-content/50">{label}</span>
        <div className="w-8 h-8 rounded-lg flex items-center justify-center bg-base-200/60">{icon}</div>
      </div>
      <div className="font-mono text-3xl font-bold tracking-tight">{value}</div>
      {sub && <div className="text-xs text-base-content/40 mt-1">{sub}</div>}
    </motion.div>
  );
}

// ── Status Pill ──────────────────────────────────────────────────────────────
function StatusPill({ icon, label, value, unit }: { icon: React.ReactNode; label: string; value: string; unit?: string }) {
  return (
    <div className="flex items-center gap-2.5 rounded-xl border border-base-300/40 bg-base-100/60 px-4 py-3">
      <div className="text-base-content/40">{icon}</div>
      <span className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{label}</span>
      <span className="font-mono text-sm font-bold">{value}{unit && <span className="text-base-content/40 ml-0.5 text-xs">{unit}</span>}</span>
    </div>
  );
}

// ── Main Page ────────────────────────────────────────────────────────────────
export default function AdminDashboard() {
  const navigate = useNavigate();
  const reducedMotion = useReducedMotion();

  const { data: todaySummary } = useUsageSummary({ since: startOfDay() });
  const { data: monthSummary } = useUsageSummary({ since: startOfMonth() });
  const { data: recentLogs, isLoading: logsLoading } = useLogs({}, 1, 10);
  const { data: channelLogs } = useLogs({ since: startOfDay() }, 1, 200);
  const { data: providers } = useProviders();
  const { data: channels } = useAllChannels();
  const { data: models } = useAllModels();
  const { data: users } = useUsers(1, 1);
  const { data: keys } = useKeys(1, 1);
  const { data: systemInfo } = useSystemInfo();

  const todayRequests = todaySummary?.reduce((sum, r) => sum + r.request_count, 0) ?? 0;
  const todayCost = todaySummary?.reduce((sum, r) => sum + r.total_cost, 0) ?? 0;
  const monthCost = monthSummary?.reduce((sum, r) => sum + r.total_cost, 0) ?? 0;
  const todayTokens = todaySummary?.reduce((sum, r) => sum + r.total_input_tokens + r.total_output_tokens, 0) ?? 0;

  const activeProviders = providers?.filter(p => p.enabled).length ?? 0;
  const activeChannels = channels?.filter(c => c.enabled).length ?? 0;
  const errorLogs = recentLogs?.items?.filter(r => r.status_code >= 400) ?? [];
  const errorRate = recentLogs?.items?.length
    ? Math.round((errorLogs.length / recentLogs.items.length) * 100)
    : 0;
  const avgLatency = recentLogs?.items?.length
    ? Math.round(recentLogs.items.reduce((sum, r) => sum + r.latency_ms, 0) / recentLogs.items.length)
    : 0;

  const topModels = [...(todaySummary ?? [])]
    .sort((a, b) => b.request_count - a.request_count)
    .slice(0, 5);

  // Aggregate channel usage from today's logs
  const channelStats = (() => {
    const items = channelLogs?.items ?? [];
    if (!items.length) return [];
    const map = new Map<string, { name: string; requests: number; errors: number; latency: number }>();
    for (const log of items) {
      const key = log.channel_id ?? '_none';
      const name = log.channel_name ?? 'Direct';
      const entry = map.get(key) ?? { name, requests: 0, errors: 0, latency: 0 };
      entry.requests++;
      if (log.status_code >= 400) entry.errors++;
      entry.latency += log.latency_ms;
      map.set(key, entry);
    }
    return [...map.values()]
      .map(c => ({ ...c, avgLatency: Math.round(c.latency / c.requests), errorRate: Math.round((c.errors / c.requests) * 100) }))
      .sort((a, b) => b.requests - a.requests);
  })();

  return (
    <div className="px-6 pb-8">
      {/* Header */}
      <motion.div
        initial={reducedMotion ? false : { opacity: 0, y: -12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={reducedMotion ? { duration: 0 } : { duration: 0.4, ease: [0.16, 1, 0.3, 1] }}
        className="mb-8 pt-8"
      >
        <h1 className="text-3xl font-black tracking-tight text-base-content leading-none mb-1">
          Admin Dashboard
        </h1>
        <p className="text-base text-base-content/50">
          System overview and real-time gateway metrics
        </p>
      </motion.div>

      {/* System Status Bar */}
      <motion.div
        initial={reducedMotion ? false : { opacity: 0, y: 12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={reducedMotion ? { duration: 0 } : { duration: 0.35, delay: 0.05, ease: [0.16, 1, 0.3, 1] }}
        className="mb-6 rounded-2xl border border-base-300/40 bg-base-100 overflow-hidden"
      >
        <div className="p-5 flex flex-wrap items-center gap-6">
          <div className="flex items-center gap-3">
            <span className="w-2 h-2 rounded-full bg-success shadow-[0_0_8px_rgba(6,214,160,0.5)]" />
            <span className="text-sm font-semibold text-base-content/70">System Online</span>
          </div>
          {systemInfo && (
            <div className="flex flex-wrap items-center gap-4 text-xs font-mono text-base-content/40">
              <span>{systemInfo.database_driver}</span>
              <span>timeout {systemInfo.upstream_timeout_secs}s</span>
              {systemInfo.audit_retention_days && <span>audit {systemInfo.audit_retention_days}d</span>}
            </div>
          )}
          <div className="ml-auto flex items-center gap-4">
            <button onClick={() => navigate('/admin/providers')} className="flex items-center gap-1.5 text-xs text-base-content/40 hover:text-accent transition-colors cursor-pointer">
              <Server className="h-3.5 w-3.5" />
              {activeProviders}/{providers?.length ?? 0} providers
            </button>
            <button onClick={() => navigate('/admin/channels')} className="flex items-center gap-1.5 text-xs text-base-content/40 hover:text-accent transition-colors cursor-pointer">
              <Network className="h-3.5 w-3.5" />
              {activeChannels}/{channels?.length ?? 0} channels
            </button>
            <button onClick={() => navigate('/admin/models')} className="flex items-center gap-1.5 text-xs text-base-content/40 hover:text-accent transition-colors cursor-pointer">
              <Cpu className="h-3.5 w-3.5" />
              {models?.length ?? 0} models
            </button>
            <button onClick={() => navigate('/admin/users')} className="flex items-center gap-1.5 text-xs text-base-content/40 hover:text-accent transition-colors cursor-pointer">
              <Users className="h-3.5 w-3.5" />
              {users?.total ?? 0} users
            </button>
            <button onClick={() => navigate('/console/keys')} className="flex items-center gap-1.5 text-xs text-base-content/40 hover:text-accent transition-colors cursor-pointer">
              <KeyRound className="h-3.5 w-3.5" />
              {keys?.total ?? 0} keys
            </button>
          </div>
        </div>
      </motion.div>

      {/* Metric Cards */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-3 mb-6">
        <MetricCard
          label="Today's Requests"
          value={todayRequests.toLocaleString()}
          icon={<MessageSquare className="h-4 w-4 text-blue-400" />}
          sub={`${todayTokens.toLocaleString()} tokens`}
          index={0}
        />
        <MetricCard
          label="Today's Cost"
          value={`$${todayCost.toFixed(4)}`}
          icon={<DollarSign className="h-4 w-4 text-emerald-400" />}
          index={1}
        />
        <MetricCard
          label="Monthly Cost"
          value={`$${monthCost.toFixed(2)}`}
          icon={<TrendingUp className="h-4 w-4 text-amber-400" />}
          sub={`${(monthSummary?.reduce((s, r) => s + r.request_count, 0) ?? 0).toLocaleString()} requests`}
          index={2}
        />
        <MetricCard
          label="Active Models"
          value={String(models?.length ?? 0)}
          icon={<Zap className="h-4 w-4 text-violet-400" />}
          sub={`${topModels.length} with traffic today`}
          index={3}
        />
      </div>

      {/* Status Pills */}
      <motion.div
        initial={reducedMotion ? false : { opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={reducedMotion ? { duration: 0 } : { duration: 0.35, delay: 0.15, ease: [0.16, 1, 0.3, 1] }}
        className="flex flex-wrap gap-3 mb-8"
      >
        <StatusPill icon={<Activity className="h-4 w-4" />} label="Avg Latency" value={String(avgLatency)} unit="ms" />
        <StatusPill
          icon={errorRate > 5 ? <AlertTriangle className="h-4 w-4 text-amber-400" /> : <TrendingUp className="h-4 w-4" />}
          label="Error Rate"
          value={String(errorRate)}
          unit="%"
        />
        <StatusPill icon={<Clock className="h-4 w-4" />} label="Recent" value={String(recentLogs?.items?.length ?? 0)} unit="reqs" />
      </motion.div>

      {/* Three columns: Top Models + Channel Usage + Recent Requests */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
        {/* Top Models */}
        <div className="lg:col-span-1">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-sm font-bold text-base-content/70">Top Models Today</h2>
            <button onClick={() => navigate('/admin/models')} className="flex items-center gap-1 text-xs text-base-content/40 hover:text-accent transition-colors cursor-pointer">
              View all <ArrowRight className="h-3 w-3" />
            </button>
          </div>
          <div className="rounded-2xl border border-base-300/40 bg-base-100 overflow-hidden">
            {topModels.length === 0 ? (
              <div className="p-8 text-center text-sm text-base-content/40">No traffic today</div>
            ) : (
              <div className="divide-y divide-base-300/20">
                {topModels.map((m, i) => {
                  const pct = todayRequests > 0 ? Math.round((m.request_count / todayRequests) * 100) : 0;
                  return (
                    <div key={m.model_name} className="px-4 py-3 flex items-center gap-3">
                      <span className="text-xs font-mono text-base-content/30 w-4 text-right">{i + 1}</span>
                      <div className="flex-1 min-w-0">
                        <div className="font-mono text-sm font-medium text-base-content/70 truncate">{m.model_name}</div>
                        <div className="mt-1 h-1 rounded-full bg-base-200/60 overflow-hidden">
                          <div className="h-full rounded-full bg-accent/50" style={{ width: `${pct}%` }} />
                        </div>
                      </div>
                      <div className="text-right shrink-0">
                        <div className="font-mono text-xs font-bold">{m.request_count.toLocaleString()}</div>
                        <div className="font-mono text-[10px] text-base-content/35">${m.total_cost.toFixed(4)}</div>
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        </div>

        {/* Channel Usage */}
        <div className="lg:col-span-1">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-sm font-bold text-base-content/70">Channel Usage Today</h2>
            <button onClick={() => navigate('/admin/channels')} className="flex items-center gap-1 text-xs text-base-content/40 hover:text-accent transition-colors cursor-pointer">
              View all <ArrowRight className="h-3 w-3" />
            </button>
          </div>
          <div className="rounded-2xl border border-base-300/40 bg-base-100 overflow-hidden">
            {channelStats.length === 0 ? (
              <div className="p-8 text-center text-sm text-base-content/40">No traffic today</div>
            ) : (
              <div className="divide-y divide-base-300/20">
                {channelStats.map((c) => {
                  const pct = channelLogs?.items?.length ? Math.round((c.requests / channelLogs.items.length) * 100) : 0;
                  return (
                    <div key={c.name} className="px-4 py-3">
                      <div className="flex items-center justify-between mb-1.5">
                        <span className="font-mono text-sm font-medium text-base-content/70 truncate">{c.name}</span>
                        <span className="font-mono text-xs font-bold">{c.requests.toLocaleString()}</span>
                      </div>
                      <div className="h-1 rounded-full bg-base-200/60 overflow-hidden mb-1.5">
                        <div className="h-full rounded-full bg-blue-400/50" style={{ width: `${pct}%` }} />
                      </div>
                      <div className="flex items-center gap-3 text-[10px] font-mono text-base-content/35">
                        <span>{c.avgLatency}ms avg</span>
                        {c.errorRate > 0 && (
                          <span className={c.errorRate > 10 ? 'text-amber-400' : ''}>{c.errorRate}% err</span>
                        )}
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        </div>

        {/* Recent Requests */}
        <div className="lg:col-span-1">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-sm font-bold text-base-content/70">Recent Requests</h2>
            <button onClick={() => navigate('/admin/logs')} className="flex items-center gap-1 text-xs text-base-content/40 hover:text-accent transition-colors cursor-pointer">
              View all <ArrowRight className="h-3 w-3" />
            </button>
          </div>
          <div className="rounded-2xl border border-base-300/40 bg-base-100 overflow-hidden">
            {logsLoading ? (
              <div className="p-5 space-y-3">
                {[...Array(5)].map((_, i) => (
                  <div key={i} className="h-10 bg-base-200/40 rounded-lg animate-pulse" />
                ))}
              </div>
            ) : (
              <div className="overflow-x-auto">
                <table className="table table-sm">
                  <thead>
                    <tr className="border-b border-base-300/40">
                      <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">Time</th>
                      <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">Model</th>
                      <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">Protocol</th>
                      <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">Status</th>
                      <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">Tokens</th>
                      <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">Latency</th>
                    </tr>
                  </thead>
                  <tbody>
                    {recentLogs?.items?.map((log) => (
                      <tr key={log.id} className="border-b border-base-200/40 hover:bg-base-200/20 transition-colors">
                        <td className="font-mono text-sm text-base-content/55">
                          {new Date(log.created_at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })}
                        </td>
                        <td className="font-mono text-sm font-medium">{log.model_name}</td>
                        <td><Badge variant={log.protocol === 'openai' ? 'blue' : 'purple'}>{log.protocol}</Badge></td>
                        <td><Badge variant={log.status_code < 400 ? 'green' : log.status_code < 500 ? 'amber' : 'red'}>{log.status_code}</Badge></td>
                        <td className="font-mono text-sm text-base-content/55">
                          {log.input_tokens ?? 0} + {log.output_tokens ?? 0}
                        </td>
                        <td className="font-mono text-sm text-base-content/55">{log.latency_ms}ms</td>
                      </tr>
                    ))}
                    {(!recentLogs?.items?.length) && (
                      <tr>
                        <td colSpan={6} className="text-center py-16 text-base-content/40 text-sm">
                          No requests yet
                        </td>
                      </tr>
                    )}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
