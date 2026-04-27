import { MessageSquare, DollarSign, Zap, TrendingUp, Activity, Clock, ArrowRight } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { useLogs } from '../hooks/useLogs';
import { useUsageSummary } from '../hooks/useUsage';
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

// ── Metric Card ──────────────────────────────────────────────────────────────
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
      transition={reducedMotion ? { duration: 0 } : { duration: 0.35, delay: index * 0.05, ease: [0.16, 1, 0.3, 1] }}
      className="relative rounded-2xl border border-base-300/40 bg-base-100 p-5 overflow-hidden"
    >
      <div className="flex items-center justify-between mb-4">
        <span className="text-xs font-semibold uppercase tracking-wider text-base-content/50">{label}</span>
        <div className="w-8 h-8 rounded-lg flex items-center justify-center bg-base-200/60">
          {icon}
        </div>
      </div>
      <div className="font-mono text-3xl font-bold tracking-tight">{value}</div>
    </motion.div>
  );
}

// ── Status Pill ──────────────────────────────────────────────────────────────
interface StatusPillProps {
  icon: React.ReactNode;
  label: string;
  value: string;
  unit?: string;
}

function StatusPill({ icon, label, value, unit }: StatusPillProps) {
  return (
    <div className="flex items-center gap-2.5 rounded-xl border border-base-300/40 bg-base-100/60 px-4 py-3">
      <div className="text-base-content/40">{icon}</div>
      <span className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{label}</span>
      <span className="font-mono text-sm font-bold">{value}{unit && <span className="text-base-content/40 ml-0.5 text-xs">{unit}</span>}</span>
    </div>
  );
}

// ── Main Page ────────────────────────────────────────────────────────────────
export default function Dashboard() {
  const navigate = useNavigate();
  const reducedMotion = useReducedMotion();
  const { data: todaySummary } = useUsageSummary({ since: startOfDay() });
  const { data: monthSummary } = useUsageSummary({ since: startOfMonth() });
  const { data: recentLogs, isLoading: logsLoading } = useLogs({}, 1, 10);

  const todayRequests = todaySummary?.reduce((sum, r) => sum + r.request_count, 0) ?? 0;
  const todayCost = todaySummary?.reduce((sum, r) => sum + r.total_cost, 0) ?? 0;
  const monthCost = monthSummary?.reduce((sum, r) => sum + r.total_cost, 0) ?? 0;
  const totalModels = new Set([...(todaySummary ?? []), ...(monthSummary ?? [])].map(r => r.model_name)).size;

  const avgLatency = recentLogs?.items?.length
    ? Math.round(recentLogs.items.reduce((sum, r) => sum + r.latency_ms, 0) / recentLogs.items.length)
    : 0;
  const successRate = recentLogs?.items?.length
    ? Math.round((recentLogs.items.filter(r => r.status_code < 400).length / recentLogs.items.length) * 100)
    : 100;

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
          Dashboard
        </h1>
        <p className="text-base text-base-content/50">
          Real-time overview of your LLM gateway activity
        </p>
      </motion.div>

      {/* Metric Cards */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-3 mb-6">
        <MetricCard
          label="Today's Requests"
          value={todayRequests.toLocaleString()}
          icon={<MessageSquare className="h-4 w-4 text-blue-400" />}
          index={0}
          reducedMotion={reducedMotion}
        />
        <MetricCard
          label="Today's Cost"
          value={`$${todayCost.toFixed(4)}`}
          icon={<DollarSign className="h-4 w-4 text-emerald-400" />}
          index={1}
          reducedMotion={reducedMotion}
        />
        <MetricCard
          label="Monthly Cost"
          value={`$${monthCost.toFixed(2)}`}
          icon={<TrendingUp className="h-4 w-4 text-amber-400" />}
          index={2}
          reducedMotion={reducedMotion}
        />
        <MetricCard
          label="Active Models"
          value={String(totalModels)}
          icon={<Zap className="h-4 w-4 text-violet-400" />}
          index={3}
          reducedMotion={reducedMotion}
        />
      </div>

      {/* Status Pills */}
      <motion.div
        initial={reducedMotion ? false : { opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={reducedMotion ? { duration: 0 } : { duration: 0.35, delay: 0.15, ease: [0.16, 1, 0.3, 1] }}
        className="flex flex-wrap gap-3 mb-8"
      >
        <StatusPill
          icon={<Activity className="h-4 w-4" />}
          label="Avg Latency"
          value={String(avgLatency)}
          unit="ms"
        />
        <StatusPill
          icon={<TrendingUp className="h-4 w-4" />}
          label="Success Rate"
          value={String(successRate)}
          unit="%"
        />
        <StatusPill
          icon={<Clock className="h-4 w-4" />}
          label="Recent"
          value={String(recentLogs?.items?.length ?? 0)}
          unit="reqs"
        />
      </motion.div>

      {/* Recent Requests */}
      <div>
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-sm font-bold text-base-content/70">Recent Requests</h2>
          <button
            onClick={() => navigate('/console/logs')}
            className="flex items-center gap-1 text-xs text-base-content/40 hover:text-accent transition-colors cursor-pointer"
          >
            View all
            <ArrowRight className="h-3 w-3" />
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
  );
}
