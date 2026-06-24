import { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  Info,
  Globe,
  Database,
  Clock,
  Timer,
  Activity,
  Coins,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useSettings, useUpdateSettings, useSystemInfo, useNatsStatus } from '../hooks/useSettings';
import { useReducedMotion } from '../hooks/useReducedMotion';
import { useCurrencyStore } from '../stores/currency';
import { Button } from '../components/ui/Button';
import { Toggle } from '../components/ui/Toggle';
import { Alert } from '../components/ui/Alert';
import { cn } from '../lib/cn';
import { apiClient } from '../api/client';

const EASE = [0.16, 1, 0.3, 1] as const;

type SettingsTab = 'general' | 'security' | 'system' | 'about';

export default function Settings() {
  const { t } = useTranslation();
  const { data: settings, isLoading } = useSettings();
  const { data: systemInfo } = useSystemInfo();
  const { data: natsStatus } = useNatsStatus();
  const updateMutation = useUpdateSettings();
  const reducedMotion = useReducedMotion();
  const { currency, setCurrency } = useCurrencyStore();

  const TABS: { key: SettingsTab; label: string }[] = [
    { key: 'general', label: t('settings.tabs.general') },
    { key: 'security', label: t('settings.tabs.security') },
    { key: 'system', label: t('settings.tabs.system') },
    { key: 'about', label: t('settings.tabs.about') },
  ];

  const [activeTab, setActiveTab] = useState<SettingsTab>('general');
  const [serverHost, setServerHost] = useState('');
  const [serverHostLoading, setServerHostLoading] = useState(false);
  const [version, setVersion] = useState('');

  useEffect(() => {
    if (settings?.server_host !== undefined) {
      setServerHost(settings.server_host ?? '');
    }
  }, [settings?.server_host]);

  useEffect(() => {
    apiClient
      .get<{ version: string }>('/version')
      .then((r) => setVersion(r.data.version))
      .catch(() => {});
  }, []);

  const handleServerHostSave = async () => {
    setServerHostLoading(true);
    try {
      await updateMutation.mutateAsync({ server_host: serverHost });
    } catch {
      // error toast handled by hook
    } finally {
      setServerHostLoading(false);
    }
  };

  const anim = (delay = 0) =>
    reducedMotion
      ? false
      : { initial: { opacity: 0, y: 12 }, animate: { opacity: 1, y: 0 }, transition: { duration: 0.35, delay, ease: EASE } };

  return (
    <div className="px-6 pb-8">
      {/* Header */}
      <motion.div
        initial={reducedMotion ? false : { opacity: 0, y: -12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={reducedMotion ? { duration: 0 } : { duration: 0.4, ease: EASE }}
        className="mb-6 pt-8"
      >
        <h1 className="text-3xl font-black tracking-tight text-base-content">{t('settings.title')}</h1>
        <p className="text-base text-base-content/50 mt-1">{t('settings.description')}</p>
      </motion.div>

      {/* Tab bar */}
      <motion.div
        {...anim(0.05)}
        className="flex items-center gap-1 border-b border-base-300/40 mb-6 overflow-x-auto"
      >
        {TABS.map((tab) => (
          <button
            key={tab.key}
            onClick={() => setActiveTab(tab.key)}
            className={cn(
              'relative px-4 py-2.5 text-sm font-medium transition-colors cursor-pointer whitespace-nowrap',
              activeTab === tab.key
                ? 'text-primary'
                : 'text-base-content/40 hover:text-base-content/60',
            )}
          >
            {tab.label}
            {activeTab === tab.key && (
              <motion.div
                layoutId="settings-tab-indicator"
                className="absolute bottom-0 left-0 right-0 h-0.5 bg-primary rounded-full"
                transition={
                  reducedMotion ? { duration: 0 } : { duration: 0.2, ease: EASE }
                }
              />
            )}
          </button>
        ))}
      </motion.div>

      {/* Tab panels */}
      <AnimatePresence mode="wait">
        <motion.div
          key={activeTab}
          initial={reducedMotion ? false : { opacity: 0, y: 8 }}
          animate={{ opacity: 1, y: 0 }}
          exit={reducedMotion ? undefined : { opacity: 0, y: -8 }}
          transition={reducedMotion ? { duration: 0 } : { duration: 0.2, ease: EASE }}
        >
          {activeTab === 'general' && (
            isLoading ? (
              <div className="flex items-center justify-center py-20">
                <span className="loading loading-spinner loading-lg text-base-content/20" />
              </div>
            ) : (
              <div className="max-w-2xl space-y-4">
                <div className="rounded-2xl border border-base-300/40 bg-base-100 overflow-hidden">
                  <div className="px-5 py-3 border-b border-base-300/60 bg-base-100/60">
                    <span className="text-[10px] font-mono font-semibold uppercase tracking-[0.18em] text-base-content/25">
                      {t('settings.general.sectionTitle')}
                    </span>
                  </div>
                  <div className="px-5 divide-y divide-base-200/50">
                    {/* Allow Registration */}
                    <div className="flex items-center justify-between gap-6 py-3">
                      <div>
                        <div className="text-sm font-medium text-base-content">{t('settings.general.allowRegistration')}</div>
                        <div className="text-xs text-base-content/40 mt-0.5">{t('settings.general.allowRegistrationDesc')}</div>
                      </div>
                      <Toggle
                        checked={settings?.allow_registration ?? false}
                        onChange={(v) => updateMutation.mutate({ allow_registration: v })}
                      />
                    </div>

                    {/* Server Host */}
                    <div className="py-3 space-y-2">
                      <div className="text-sm font-medium text-base-content">{t('settings.general.serverHost')}</div>
                      <div className="text-xs text-base-content/40">{t('settings.general.serverHostDesc')}</div>
                      <div className="flex gap-2">
                        <input
                          type="text"
                          value={serverHost}
                          onChange={(e) => setServerHost(e.target.value)}
                          placeholder="http://localhost:8080"
                          spellCheck={false}
                          className="flex-1 h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm text-base-content placeholder:text-base-content/25 focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
                        />
                        <Button
                          variant="primary"
                          size="sm"
                          loading={serverHostLoading}
                          disabled={serverHost === (settings?.server_host ?? '')}
                          onClick={handleServerHostSave}
                        >
                          {t('common.save')}
                        </Button>
                      </div>
                    </div>

                    {/* Currency */}
                    <div className="flex items-center justify-between gap-6 py-3">
                      <div className="flex items-center gap-3">
                        <Coins className="h-4 w-4 text-base-content/40" />
                        <div>
                          <div className="text-sm font-medium text-base-content">{t('settings.general.currency')}</div>
                          <div className="text-xs text-base-content/40 mt-0.5">{t('settings.general.currencyDesc')}</div>
                        </div>
                      </div>
                      <div className="flex items-center gap-1 bg-base-200/50 rounded-lg p-0.5">
                        {(['USD', 'CNY'] as const).map((c) => (
                          <button
                            key={c}
                            onClick={() => {
                              updateMutation.mutate({ currency: c });
                              setCurrency(c);
                            }}
                            className={cn(
                              'px-3 py-1.5 text-xs font-medium rounded-md transition-colors cursor-pointer',
                              currency === c
                                ? 'bg-base-100 text-base-content shadow-sm'
                                : 'text-base-content/40 hover:text-base-content/60',
                            )}
                          >
                            {c === 'USD' ? '$ USD' : '¥ CNY'}
                          </button>
                        ))}
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            )
          )}

          {activeTab === 'security' && (
            <div className="max-w-2xl space-y-4">
              <div className="rounded-2xl border border-base-300/40 bg-base-100 overflow-hidden">
                <div className="px-5 py-3 border-b border-base-300/60 bg-base-100/60">
                  <span className="text-[10px] font-mono font-semibold uppercase tracking-[0.18em] text-base-content/25">
                    {t('settings.security.sectionTitle')}
                  </span>
                </div>
                <div className="px-5 divide-y divide-base-200/50">
                  {/* Log Request Body */}
                  <div className="flex items-center justify-between gap-6 py-3">
                    <div>
                      <div className="text-sm font-medium text-base-content">{t('settings.security.logRequestBody')}</div>
                      <div className="text-xs text-base-content/40 mt-0.5">{t('settings.security.logRequestBodyDesc')}</div>
                    </div>
                    <Toggle
                      checked={settings?.audit_log_request ?? true}
                      onChange={(v) => updateMutation.mutate({ audit_log_request: v })}
                    />
                  </div>

                  {/* Log Response Body */}
                  <div className="flex items-center justify-between gap-6 py-3">
                    <div>
                      <div className="text-sm font-medium text-base-content">{t('settings.security.logResponseBody')}</div>
                      <div className="text-xs text-base-content/40 mt-0.5">{t('settings.security.logResponseBodyDesc')}</div>
                    </div>
                    <Toggle
                      checked={settings?.audit_log_response ?? true}
                      onChange={(v) => updateMutation.mutate({ audit_log_response: v })}
                    />
                  </div>
                </div>
              </div>
              <Alert variant="info" className="text-xs">
                {t('settings.security.storageNotice')}
              </Alert>
            </div>
          )}

          {activeTab === 'system' && (
            <div className="max-w-2xl space-y-4">
              <div className="rounded-2xl border border-base-300/40 bg-base-100 overflow-hidden">
                <div className="px-5 py-3 border-b border-base-300/60 bg-base-100/60">
                  <span className="text-[10px] font-mono font-semibold uppercase tracking-[0.18em] text-base-content/25">
                    {t('settings.system.sectionTitle')}
                  </span>
                </div>
                <div className="p-5">
                  <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                    {systemInfo ? [
                      { label: t('settings.system.serverBindAddress'), value: systemInfo.server_bind_address, Icon: Globe },
                      { label: t('settings.system.databaseDriver'), value: systemInfo.database_driver, Icon: Database },
                      { label: t('settings.system.rateLimitWindow'), value: `${systemInfo.rate_limit_window_secs}s`, Icon: Clock },
                      { label: t('settings.system.rateLimitFlushInterval'), value: `${systemInfo.rate_limit_flush_interval_secs}s`, Icon: Timer },
                      { label: t('settings.system.upstreamTimeout'), value: `${systemInfo.upstream_timeout_secs}s`, Icon: Timer },
                      { label: t('settings.system.auditRetention'), value: systemInfo.audit_retention_days != null ? t('settings.system.days', { count: systemInfo.audit_retention_days }) : '--', Icon: Clock },
                    ].map(({ label, value, Icon }) => (
                      <div key={label} className="flex items-center gap-3">
                        <div className="w-8 h-8 rounded-lg bg-base-200/60 flex items-center justify-center shrink-0">
                          <Icon className="h-4 w-4 text-base-content/40" />
                        </div>
                        <div>
                          <div className="text-xs text-base-content/40">{label}</div>
                          <div className="text-sm font-mono font-medium">{value}</div>
                        </div>
                      </div>
                    )) : (
                      <div className="col-span-2 text-sm text-base-content/40">{t('common.loading')}</div>
                    )}
                  </div>
                </div>
              </div>

              {natsStatus && (
                <div className="rounded-2xl border border-base-300/40 bg-base-100 overflow-hidden">
                  <div className="px-5 py-3 border-b border-base-300/60 bg-base-100/60">
                    <span className="text-[10px] font-mono font-semibold uppercase tracking-[0.18em] text-base-content/25">
                      {t('settings.nats.sectionTitle')}
                    </span>
                  </div>
                  <div className="p-5 space-y-4">
                    {natsStatus?.streams?.map((stream) => (
                      <div key={stream.name} className="bg-base-200/40 rounded-xl p-4">
                        <div className="flex items-center gap-2 mb-3">
                          <Activity className="h-4 w-4 text-success" />
                          <span className="text-sm font-semibold font-mono">{stream.name}</span>
                        </div>
                        <div className="grid grid-cols-2 sm:grid-cols-5 gap-3">
                          <div>
                            <div className="text-[10px] text-base-content/40 uppercase tracking-wider">{t('settings.nats.messages')}</div>
                            <div className="text-sm font-mono font-medium">{stream.messages.toLocaleString()}</div>
                          </div>
                          <div>
                            <div className="text-[10px] text-base-content/40 uppercase tracking-wider">{t('settings.nats.pending')}</div>
                            <div className="text-sm font-mono font-medium text-warning">{stream.pending_messages.toLocaleString()}</div>
                          </div>
                          <div>
                            <div className="text-[10px] text-base-content/40 uppercase tracking-wider">{t('settings.nats.size')}</div>
                            <div className="text-sm font-mono font-medium">
                              {stream.bytes < 1024 ? `${stream.bytes} B`
                                : stream.bytes < 1048576 ? `${(stream.bytes / 1024).toFixed(1)} KB`
                                : stream.bytes < 1073741824 ? `${(stream.bytes / 1048576).toFixed(1)} MB`
                                : `${(stream.bytes / 1073741824).toFixed(2)} GB`}
                            </div>
                          </div>
                          <div>
                            <div className="text-[10px] text-base-content/40 uppercase tracking-wider">{t('settings.nats.consumers')}</div>
                            <div className="text-sm font-mono font-medium">{stream.consumer_count}</div>
                          </div>
                          <div>
                            <div className="text-[10px] text-base-content/40 uppercase tracking-wider">{t('settings.nats.retention')}</div>
                            <div className="text-sm font-mono font-medium">{t('settings.nats.days', { count: Math.round(stream.max_age_secs / 86400) })}</div>
                          </div>
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              <Alert variant="warning" className="text-xs">
                {t('settings.system.configNotice')}
              </Alert>
            </div>
          )}

          {activeTab === 'about' && (
            <div className="max-w-2xl space-y-4">
              <div className="rounded-2xl border border-base-300/40 bg-base-100 overflow-hidden">
                <div className="px-5 py-3 border-b border-base-300/60 bg-base-100/60">
                  <span className="text-[10px] font-mono font-semibold uppercase tracking-[0.18em] text-base-content/25">
                    {t('settings.about.sectionTitle')}
                  </span>
                </div>
                <div className="p-5">
                  <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                    <div className="flex items-center gap-3">
                      <div className="w-8 h-8 rounded-lg bg-base-200/60 flex items-center justify-center shrink-0">
                        <Info className="h-4 w-4 text-base-content/40" />
                      </div>
                      <div>
                        <div className="text-xs text-base-content/40">{t('settings.about.version')}</div>
                        <div className="text-sm font-mono font-medium">
                          {version || '—'}
                        </div>
                      </div>
                    </div>
                    <div className="flex items-center gap-3">
                      <div className="w-8 h-8 rounded-lg bg-base-200/60 flex items-center justify-center shrink-0">
                        <Database className="h-4 w-4 text-base-content/40" />
                      </div>
                      <div>
                        <div className="text-xs text-base-content/40">{t('settings.about.database')}</div>
                        <div className="text-sm font-mono font-medium">{systemInfo?.database_driver || '—'}</div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          )}
        </motion.div>
      </AnimatePresence>
    </div>
  );
}
