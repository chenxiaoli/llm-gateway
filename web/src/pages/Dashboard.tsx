import { useState } from 'react';
import { MessageSquare, DollarSign, Zap, TrendingUp, Activity, Clock, ArrowRight, Wallet, AlertTriangle } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip } from 'recharts';
import { useDailyUsage } from '../hooks/useUsage';
import { useLogs } from '../hooks/useLogs';
import { useUsageSummary } from '../hooks/useUsage';
import { useMyBalance } from '../hooks/useAccounts';
import { useReducedMotion } from '../hooks/useReducedMotion';
import { useAuthStore } from '../stores/authStore';
import { useCurrencyStore, formatCurrency } from '../stores/currency';
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

// ── Status Pill ──────────────────────────────────────────���───────────────────
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
  const { t } = useTranslation();
  const navigate = useNavigate();
  const reducedMotion = useReducedMotion();
  const user = useAuthStore((s) => s.user);
  const symbol = useCurrencyStore((s) => s.symbol);
  const { data: todaySummary } = useUsageSummary({ since: startOfDay() });
  const { data: monthSummary } = useUsageSummary({ since: startOfMonth() });
  const { data: recentLogs, isLoading: logsLoading } = useLogs({}, 1, 10);
  const { data: myBalance } = useMyBalance(1, 1);

  const [chartRange, setChartRange] = useState<7 | 30>(7);
  const sinceDate = new Date();
  sinceDate.setDate(sinceDate.getDate() - chartRange);
  sinceDate.setHours(0, 0, 0, 0);
  const tz = Intl.DateTimeFormat().resolvedOptions().timeZone;
  const dailyQuery = useDailyUsage({
    since: sinceDate.toISOString(),
    tz,
  });
  const dailyData = dailyQuery.data;

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
          {t('dashboard.title')}
        </h1>
        <p className="text-base text-base-content/50">
          {t('dashboard.description')}
        </p>
      </motion.div>

      {/* Account Balance */}
      {myBalance && (
        <motion.div
          initial={reducedMotion ? false : { opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={reducedMotion ? { duration: 0 } : { duration: 0.35, delay: 0.05, ease: [0.16, 1, 0.3, 1] }}
          className="mb-6 rounded-2xl border border-base-300/40 bg-base-100 overflow-hidden"
        >
          <div className="p-5 flex items-center justify-between">
            <div className="flex items-center gap-4">
              <div className={`w-10 h-10 rounded-xl flex items-center justify-center shrink-0 ${
                myBalance.balance <= myBalance.threshold ? 'bg-amber-500/10' : 'bg-primary/10'
              }`}>
                {myBalance.balance <= myBalance.threshold
                  ? <AlertTriangle className="h-5 w-5 text-amber-500" />
                  : <Wallet className="h-5 w-5 text-primary" />
                }
              </div>
              <div>
                <span className="text-xs font-semibold uppercase tracking-wider text-base-content/50">{t('dashboard.accountBalance')}</span>
                <div className="font-mono text-3xl font-bold tracking-tight mt-0.5">
                  {formatCurrency(myBalance.balance, symbol, 4)}
                </div>
              </div>
            </div>
            <div className="flex items-center gap-3">
              {myBalance.balance <= myBalance.threshold && (
                <Badge variant="amber">{t('dashboard.lowBalance')}</Badge>
              )}
              {user && (
                <button
                  onClick={() => navigate('/console/account')}
                  className="flex items-center gap-1 text-xs text-base-content/40 hover:text-accent transition-colors cursor-pointer"
                >
                  {t('dashboard.viewDetails')}
                  <ArrowRight className="h-3 w-3" />
                </button>
              )}
            </div>
          </div>
        </motion.div>
      )}

      {/* Metric Cards */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-3 mb-6">
        <MetricCard
          label={t('dashboard.stats.todayRequests')}
          value={todayRequests.toLocaleString()}
          icon={<MessageSquare className="h-4 w-4 text-blue-400" />}
          index={0}
          reducedMotion={reducedMotion}
        />
        <MetricCard
          label={t('dashboard.stats.todayCost')}
          value={formatCurrency(todayCost, symbol, 4)}
          icon={<DollarSign className="h-4 w-4 text-emerald-400" />}
          index={1}
          reducedMotion={reducedMotion}
        />
        <MetricCard
          label={t('dashboard.stats.monthlyCost')}
          value={formatCurrency(monthCost, symbol, 2)}
          icon={<TrendingUp className="h-4 w-4 text-amber-400" />}
          index={2}
          reducedMotion={reducedMotion}
        />
        <MetricCard
          label={t('dashboard.stats.activeModels')}
          value={String(totalModels)}
          icon={<Zap className="h-4 w-4 text-violet-400" />}
          index={3}
          reducedMotion={reducedMotion}
        />
      </div>

      {/* Daily Usage Chart */}
      <div className="rounded-xl border border-base-300/40 bg-base-100 p-5 mb-6">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-sm font-semibold text-base-content/70">{t('dashboard.chartTitle')}</h3>
          <div className="flex bg-base-200/60 rounded-lg p-0.5">
            <button
              onClick={() => setChartRange(7)}
              className={`px-3 py-1 rounded-md text-xs font-medium transition-all duration-150 ${
                chartRange === 7 ? 'bg-base-100 text-base-content shadow-sm' : 'text-base-content/40 hover:text-base-content/60'
              }`}
            >
              {t('dashboard.last7Days')}
            </button>
            <button
              onClick={() => setChartRange(30)}
              className={`px-3 py-1 rounded-md text-xs font-medium transition-all duration-150 ${
                chartRange === 30 ? 'bg-base-100 text-base-content shadow-sm' : 'text-base-content/40 hover:text-base-content/60'
              }`}
            >
              {t('dashboard.last30Days')}
            </button>
          </div>
        </div>
        <div style={{ width: '100%', height: 280 }}>
          <LineChart width={800} height={280} data={dailyData || []} margin={{ top: 5, right: 20, bottom: 30, left: 10 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="rgba(128,128,128,0.15)" />
            <XAxis
              dataKey="date"
              stroke="rgba(128,128,128,0.4)"
              tick={{ fontSize: 11, fill: 'rgba(128,128,128,0.8)' }}
              tickFormatter={(v: string) => v.slice(5)}
            />
            <YAxis
              stroke="rgba(128,128,128,0.4)"
              tick={{ fontSize: 11, fill: 'rgba(128,128,128,0.8)' }}
              tickFormatter={(v: number) => v >= 1000000 ? `${(v / 1000000).toFixed(1)}M` : v >= 1000 ? `${(v / 1000).toFixed(0)}K` : String(v)}
            />
            <Tooltip
              contentStyle={{
                backgroundColor: 'rgba(30,30,50,0.95)',
                border: '1px solid rgba(255,255,255,0.1)',
                borderRadius: 8,
                fontSize: 12,
                color: '#fff',
              }}
              formatter={(value: number, name: string) => {
                const labels: Record<string, string> = {
                  total_weighted_tokens: t('dashboard.weightedTokens'),
                  total_input_tokens: t('dashboard.inputTokens'),
                  total_output_tokens: t('dashboard.outputTokens'),
                  total_cache_read_tokens: t('dashboard.cacheReadTokens'),
                  total_cache_creation_tokens: t('dashboard.cacheCreationTokens'),
                };
                return [value.toLocaleString(), labels[name] || name];
              }}
              labelFormatter={(label: string) => label}
            />
            <Line
              type="monotone"
              dataKey="total_weighted_tokens"
              stroke="#6366f1"
              strokeWidth={2}
              dot={false}
              activeDot={{ r: 4, strokeWidth: 0, fill: '#6366f1' }}
            />
          </LineChart>
        </div>
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
          label={t('dashboard.stats.avgLatency')}
          value={String(avgLatency)}
          unit={t('dashboard.units.ms')}
        />
        <StatusPill
          icon={<TrendingUp className="h-4 w-4" />}
          label={t('dashboard.stats.successRate')}
          value={String(successRate)}
          unit={t('dashboard.units.percent')}
        />
        <StatusPill
          icon={<Clock className="h-4 w-4" />}
          label={t('dashboard.stats.recent')}
          value={String(recentLogs?.items?.length ?? 0)}
          unit={t('dashboard.units.reqs')}
        />
      </motion.div>

      {/* Recent Requests */}
      <div>
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-sm font-bold text-base-content/70">{t('dashboard.recentRequests')}</h2>
          <button
            onClick={() => navigate('/admin/logs')}
            className="flex items-center gap-1 text-xs text-base-content/40 hover:text-accent transition-colors cursor-pointer"
          >
            {t('dashboard.viewAll')}
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
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('dashboard.table.time')}</th>
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('dashboard.table.model')}</th>
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('dashboard.table.protocol')}</th>
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('dashboard.table.status')}</th>
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('dashboard.table.tokens')}</th>
                    <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('dashboard.table.latency')}</th>
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
                        {t('dashboard.noRequests')}
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
