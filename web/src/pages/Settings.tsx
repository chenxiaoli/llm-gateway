import { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  Info,
  Globe,
  Database,
  Clock,
  Timer,
  ExternalLink,
} from 'lucide-react';
import { useSettings, useUpdateSettings } from '../hooks/useSettings';
import { useReducedMotion } from '../hooks/useReducedMotion';
import { Button } from '../components/ui/Button';
import { Toggle } from '../components/ui/Toggle';
import { Alert } from '../components/ui/Alert';
import { cn } from '../lib/cn';
import { apiClient } from '../api/client';

const EASE = [0.16, 1, 0.3, 1] as const;

type SettingsTab = 'general' | 'security' | 'system' | 'about';

const TABS: { key: SettingsTab; label: string }[] = [
  { key: 'general', label: 'General' },
  { key: 'security', label: 'Security & Audit' },
  { key: 'system', label: 'System Info' },
  { key: 'about', label: 'About' },
];

const STATIC_CONFIG = [
  { label: 'Server Bind Address', value: '0.0.0.0:8080', Icon: Globe },
  { label: 'Database Driver', value: 'SQLite', Icon: Database },
  { label: 'Rate Limit Window', value: '60s', Icon: Clock },
  { label: 'Rate Limit Flush Interval', value: '30s', Icon: Timer },
  { label: 'Upstream Timeout', value: '30s', Icon: Timer },
  { label: 'Audit Retention', value: '90 days', Icon: Clock },
] as const;

export default function Settings() {
  const { data: settings, isLoading } = useSettings();
  const updateMutation = useUpdateSettings();
  const reducedMotion = useReducedMotion();

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
        <h1 className="text-3xl font-black tracking-tight text-base-content">Settings</h1>
        <p className="text-base text-base-content/50 mt-1">Gateway configuration</p>
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
                      GENERAL SETTINGS
                    </span>
                  </div>
                  <div className="px-5 divide-y divide-base-200/50">
                    {/* Allow Registration */}
                    <div className="flex items-center justify-between gap-6 py-3">
                      <div>
                        <div className="text-sm font-medium text-base-content">Allow Registration</div>
                        <div className="text-xs text-base-content/40 mt-0.5">Permit new users to register on the login page</div>
                      </div>
                      <Toggle
                        checked={settings?.allow_registration ?? false}
                        onChange={(v) => updateMutation.mutate({ allow_registration: v })}
                      />
                    </div>

                    {/* Server Host */}
                    <div className="py-3 space-y-2">
                      <div className="text-sm font-medium text-base-content">Server Host</div>
                      <div className="text-xs text-base-content/40">Gateway base URL used by the proxy endpoint</div>
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
                          Save
                        </Button>
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
                    AUDIT LOGGING
                  </span>
                </div>
                <div className="px-5 divide-y divide-base-200/50">
                  {/* Log Request Body */}
                  <div className="flex items-center justify-between gap-6 py-3">
                    <div>
                      <div className="text-sm font-medium text-base-content">Log Request Body</div>
                      <div className="text-xs text-base-content/40 mt-0.5">Capture and store upstream request payloads in audit logs</div>
                    </div>
                    <Toggle
                      checked={settings?.audit_log_request ?? true}
                      onChange={(v) => updateMutation.mutate({ audit_log_request: v })}
                    />
                  </div>

                  {/* Log Response Body */}
                  <div className="flex items-center justify-between gap-6 py-3">
                    <div>
                      <div className="text-sm font-medium text-base-content">Log Response Body</div>
                      <div className="text-xs text-base-content/40 mt-0.5">Capture and store upstream response payloads in audit logs</div>
                    </div>
                    <Toggle
                      checked={settings?.audit_log_response ?? true}
                      onChange={(v) => updateMutation.mutate({ audit_log_response: v })}
                    />
                  </div>
                </div>
              </div>
              <Alert variant="info" className="text-xs">
                Enabling request/response body logging may increase storage usage. Audit logs are retained according to the configured retention period.
              </Alert>
            </div>
          )}

          {activeTab === 'system' && (
            <div className="max-w-2xl space-y-4">
              <div className="rounded-2xl border border-base-300/40 bg-base-100 overflow-hidden">
                <div className="px-5 py-3 border-b border-base-300/60 bg-base-100/60">
                  <span className="text-[10px] font-mono font-semibold uppercase tracking-[0.18em] text-base-content/25">
                    INFRASTRUCTURE CONFIGURATION
                  </span>
                </div>
                <div className="p-5">
                  <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                    {STATIC_CONFIG.map(({ label, value, Icon }) => (
                      <div key={label} className="flex items-center gap-3">
                        <div className="w-8 h-8 rounded-lg bg-base-200/60 flex items-center justify-center shrink-0">
                          <Icon className="h-4 w-4 text-base-content/40" />
                        </div>
                        <div>
                          <div className="text-xs text-base-content/40">{label}</div>
                          <div className="text-sm font-mono font-medium">{value}</div>
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              </div>
              <Alert variant="warning" className="text-xs">
                These values are derived from config.toml. Changes require editing the configuration file and restarting the gateway.
              </Alert>
            </div>
          )}

          {activeTab === 'about' && (
            <div className="max-w-2xl space-y-4">
              <div className="rounded-2xl border border-base-300/40 bg-base-100 overflow-hidden">
                <div className="px-5 py-3 border-b border-base-300/60 bg-base-100/60">
                  <span className="text-[10px] font-mono font-semibold uppercase tracking-[0.18em] text-base-content/25">
                    GATEWAY INFORMATION
                  </span>
                </div>
                <div className="p-5">
                  <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                    <div className="flex items-center gap-3">
                      <div className="w-8 h-8 rounded-lg bg-base-200/60 flex items-center justify-center shrink-0">
                        <Info className="h-4 w-4 text-base-content/40" />
                      </div>
                      <div>
                        <div className="text-xs text-base-content/40">Version</div>
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
                        <div className="text-xs text-base-content/40">Database</div>
                        <div className="text-sm font-mono font-medium">SQLite</div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>

              <div className="rounded-2xl border border-base-300/40 bg-base-100 overflow-hidden">
                <div className="px-5 py-3 border-b border-base-300/60 bg-base-100/60">
                  <span className="text-[10px] font-mono font-semibold uppercase tracking-[0.18em] text-base-content/25">
                    LINKS
                  </span>
                </div>
                <div className="p-3">
                  <a
                    href="https://github.com/chenxiaoli/llm-gateway"
                    target="_blank"
                    rel="noopener noreferrer"
                    className="flex items-center gap-3 p-3 rounded-lg hover:bg-base-200/40 transition-colors"
                  >
                    <div className="w-8 h-8 rounded-lg bg-base-200/60 flex items-center justify-center shrink-0">
                      <svg className="h-4 w-4 text-base-content/40" viewBox="0 0 24 24" fill="currentColor">
                        <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z" />
                      </svg>
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className="text-sm font-medium">GitHub Repository</div>
                      <div className="text-xs text-base-content/40">Source code, issues, documentation</div>
                    </div>
                    <ExternalLink className="h-4 w-4 text-base-content/20 shrink-0" />
                  </a>
                </div>
              </div>
            </div>
          )}
        </motion.div>
      </AnimatePresence>
    </div>
  );
}
