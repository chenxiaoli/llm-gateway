import { MessageSquare, DollarSign, Zap, TrendingUp, Activity, Server, Network, Users, Cpu, KeyRound, ArrowRight, Clock, AlertTriangle } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { useUsageSummary, useChannelUsageSummary } from '../hooks/useUsage';
import { useLogs } from '../hooks/useLogs';
import { useProviders } from '../hooks/useProviders';
import { useAllChannels } from '../hooks/useChannels';
import { useAllModels } from '../hooks/useModels';
import { useMembers } from '../hooks/useMembers';
import { useKeys } from '../hooks/useKeys';
import { useSystemInfo, useNatsStatus } from '../hooks/useSettings';
import { useReducedMotion } from '../hooks/useReducedMotion';
import { useCurrencyStore, formatCurrency } from '../stores/currency';
import { useAuthStore } from '../stores/authStore';
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
  const { t } = useTranslation();
  const navigate = useNavigate();
  const reducedMotion = useReducedMotion();
  const symbol = useCurrencyStore((s) => s.symbol);
  const slug = useAuthStore((s) => s.currentOrg?.slug);

  const { data: todaySummary } = useUsageSummary({ since: startOfDay() });
  const { data: monthSummary } = useUsageSummary({ since: startOfMonth() });
  const { data: recentLogs, isLoading: logsLoading } = useLogs({}, 1, 10);
  const { data: channelUsage } = useChannelUsageSummary({ since: startOfDay() });
  const { data: providers } = useProviders();
  const { data: channels } = useAllChannels();
  const { data: models } = useAllModels();
  const { data: members } = useMembers();
  const { data: keys } = useKeys(1, 1);
  const { data: systemInfo } = useSystemInfo();
  const { data: natsStatus } = useNatsStatus();

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

  const channelStats = channelUsage ?? [];

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
          {t('adminDashboard.title')}
        </h1>
        <p className="text-base text-base-content/50">
          {t('adminDashboard.description')}
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
            <span className="text-sm font-semibold text-base-content/70">{t('adminDashboard.systemOnline')}</span>
          </div>
          {systemInfo && (
            <div className="flex flex-wrap items-center gap-4 text-xs font-mono text-base-content/40">
              <span>{systemInfo.database_driver}</span>
              <span>{t('adminDashboard.timeout', { seconds: systemInfo.upstream_timeout_secs })}</span>
              {systemInfo.audit_retention_days && <span>{t('adminDashboard.auditRetention', { days: systemInfo.audit_retention_days })}</span>}
            </div>
          )}
          <div className="ml-auto flex items-center gap-4">
            <button onClick={() => navigate(slug ? `/${slug}/admin/providers` : '/login')} className="flex items-center gap-1.5 text-xs text-base-content/40 hover:text-accent transition-colors cursor-pointer">
              <Server className="h-3.5 w-3.5" />
              {t('adminDashboard.providersCount', { active: activeProviders, total: providers?.length ?? 0 })}
            </button>
            <button onClick={() => navigate(slug ? `/${slug}/admin/channels` : '/login')} className="flex items-center gap-1.5 text-xs text-base-content/40 hover:text-accent transition-colors cursor-pointer">
              <Network className="h-3.5 w-3.5" />
              {t('adminDashboard.channelsCount', { active: activeChannels, total: channels?.length ?? 0 })}
            </button>
            <button onClick={() => navigate(slug ? `/${slug}/admin/models` : '/login')} className="flex items-center gap-1.5 text-xs text-base-content/40 hover:text-accent transition-colors cursor-pointer">
              <Cpu className="h-3.5 w-3.5" />
              {t('adminDashboard.modelsCount', { count: models?.length ?? 0 })}
            </button>
            <button onClick={() => navigate(slug ? `/${slug}/members` : '/login')} className="flex items-center gap-1.5 text-xs text-base-content/40 hover:text-accent transition-colors cursor-pointer">
              <Users className="h-3.5 w-3.5" />
              {t('adminDashboard.membersCount', { count: members?.length ?? 0 })}
            </button>
            <button onClick={() => navigate(slug ? `/${slug}/keys` : '/login')} className="flex items-center gap-1.5 text-xs text-base-content/40 hover:text-accent transition-colors cursor-pointer">
              <KeyRound className="h-3.5 w-3.5" />
              {t('adminDashboard.keysCount', { count: keys?.total ?? 0 })}
            </button>
          </div>
        </div>
      </motion.div>

      {/* Metric Cards */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-3 mb-6">
        <MetricCard
          label={t('adminDashboard.stats.todayRequests')}
          value={todayRequests.toLocaleString()}
          icon={<MessageSquare className="h-4 w-4 text-blue-400" />}
          sub={t('adminDashboard.stats.todayTokensSub', { count: todayTokens.toLocaleString() })}
          index={0}
        />
        <MetricCard
          label={t('adminDashboard.stats.todayCost')}
          value={formatCurrency(todayCost, symbol, 4)}
          icon={<DollarSign className="h-4 w-4 text-emerald-400" />}
          index={1}
        />
        <MetricCard
          label={t('adminDashboard.stats.monthlyCost')}
          value={formatCurrency(monthCost, symbol, 2)}
          icon={<TrendingUp className="h-4 w-4 text-amber-400" />}
          sub={t('adminDashboard.stats.monthlyRequestsSub', { count: (monthSummary?.reduce((s, r) => s + r.request_count, 0) ?? 0).toLocaleString() })}
          index={2}
        />
        <MetricCard
          label={t('adminDashboard.stats.activeModels')}
          value={String(models?.length ?? 0)}
          icon={<Zap className="h-4 w-4 text-violet-400" />}
          sub={t('adminDashboard.stats.trafficTodaySub', { count: topModels.length })}
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
        <StatusPill icon={<Activity className="h-4 w-4" />} label={t('adminDashboard.stats.avgLatency')} value={String(avgLatency)} unit={t('adminDashboard.units.ms')} />
        <StatusPill
          icon={errorRate > 5 ? <AlertTriangle className="h-4 w-4 text-amber-400" /> : <TrendingUp className="h-4 w-4" />}
          label={t('adminDashboard.stats.errorRate')}
          value={String(errorRate)}
          unit={t('adminDashboard.units.percent')}
        />
        <StatusPill icon={<Clock className="h-4 w-4" />} label={t('adminDashboard.stats.recent')} value={String(recentLogs?.items?.length ?? 0)} unit={t('adminDashboard.units.reqs')} />
        <StatusPill icon={<Activity className="h-4 w-4" />} label={t('adminDashboard.stats.natsUsage')} value={String(natsStatus?.streams?.find(s => s.name === 'LLM_GATEWAY_USAGE')?.pending_messages ?? 0)} unit={(natsStatus?.streams?.find(s => s.name === 'LLM_GATEWAY_USAGE')?.pending_messages ?? 0) > 0 ? '⚠' : '✓'} />
        <StatusPill icon={<Activity className="h-4 w-4" />} label={t('adminDashboard.stats.natsAudit')} value={String(natsStatus?.streams?.find(s => s.name === 'LLM_GATEWAY_AUDIT')?.pending_messages ?? 0)} unit={(natsStatus?.streams?.find(s => s.name === 'LLM_GATEWAY_AUDIT')?.pending_messages ?? 0) > 0 ? '⚠' : '✓'} />
      </motion.div>

      {/* Three columns: Top Models + Channel Usage + Recent Requests */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
        {/* Top Models */}
        <div className="lg:col-span-1">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-sm font-bold text-base-content/70">{t('adminDashboard.topModelsToday')}</h2>
            <button onClick={() => navigate(slug ? `/${slug}/admin/models` : '/login')} className="flex items-center gap-1 text-xs text-base-content/40 hover:text-accent transition-colors cursor-pointer">
              {t('adminDashboard.viewAll')} <ArrowRight className="h-3 w-3" />
            </button>
          </div>
          <div className="rounded-2xl border border-base-300/40 bg-base-100 overflow-hidden">
            {topModels.length === 0 ? (
              <div className="p-8 text-center text-sm text-base-content/40">{t('adminDashboard.noTrafficToday')}</div>
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
                        <div className="font-mono text-[10px] text-base-content/35">{formatCurrency(m.total_cost, symbol, 4)}</div>
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
            <h2 className="text-sm font-bold text-base-content/70">{t('adminDashboard.channelUsageToday')}</h2>
            <button onClick={() => navigate(slug ? `/${slug}/admin/channels` : '/login')} className="flex items-center gap-1 text-xs text-base-content/40 hover:text-accent transition-colors cursor-pointer">
              {t('adminDashboard.viewAll')} <ArrowRight className="h-3 w-3" />
            </button>
          </div>
          <div className="rounded-2xl border border-base-300/40 bg-base-100 overflow-hidden">
            {channelStats.length === 0 ? (
              <div className="p-8 text-center text-sm text-base-content/40">{t('adminDashboard.noTrafficToday')}</div>
            ) : (
              <div className="divide-y divide-base-300/20">
                {channelStats.map((c) => {
                  const totalReqs = channelStats.reduce((s, x) => s + x.total_requests, 0);
                  const pct = totalReqs > 0 ? Math.round((c.total_requests / totalReqs) * 100) : 0;
                  return (
                    <div key={c.channel_id ?? '_none'} className="px-4 py-3">
                      <div className="flex items-center justify-between mb-1.5">
                        <span className="font-mono text-sm font-medium text-base-content/70 truncate">{c.channel_name ?? t('adminDashboard.direct')}</span>
                        <span className="font-mono text-xs font-bold">{c.total_requests.toLocaleString()}</span>
                      </div>
                      <div className="h-1 rounded-full bg-base-200/60 overflow-hidden mb-1.5">
                        <div className="h-full rounded-full bg-blue-400/50" style={{ width: `${pct}%` }} />
                      </div>
                      <div className="flex items-center gap-3 text-[10px] font-mono text-base-content/35">
                        <span>{((c.total_input_tokens ?? 0) + (c.total_output_tokens ?? 0)).toLocaleString()} {t('adminDashboard.units.tokens')}</span>
                        <span>{formatCurrency(c.total_cost, symbol, 4)}</span>
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
            <h2 className="text-sm font-bold text-base-content/70">{t('adminDashboard.recentRequests')}</h2>
            <button onClick={() => navigate(slug ? `/${slug}/admin/logs` : '/login')} className="flex items-center gap-1 text-xs text-base-content/40 hover:text-accent transition-colors cursor-pointer">
              {t('adminDashboard.viewAll')} <ArrowRight className="h-3 w-3" />
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
                      <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('adminDashboard.table.time')}</th>
                      <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('adminDashboard.table.model')}</th>
                      <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('adminDashboard.table.protocol')}</th>
                      <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('adminDashboard.table.status')}</th>
                      <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('adminDashboard.table.tokens')}</th>
                      <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('adminDashboard.table.latency')}</th>
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
                          {t('adminDashboard.noRequests')}
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
