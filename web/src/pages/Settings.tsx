import { useState, useEffect } from 'react';
import { useSettings, useUpdateSettings } from '../hooks/useSettings';
import { Button } from '../components/ui/Button';
import { Toggle } from '../components/ui/Toggle';
import { toast } from 'sonner';
import { motion } from 'framer-motion';

export default function Settings() {
  const { data: settings, isLoading } = useSettings();
  const updateMutation = useUpdateSettings();
  const [serverHost, setServerHost] = useState('');
  const [serverHostLoading, setServerHostLoading] = useState(false);

  useEffect(() => {
    if (settings?.server_host !== undefined) {
      setServerHost(settings.server_host ?? '');
    }
  }, [settings?.server_host]);

  const handleServerHostSave = async () => {
    setServerHostLoading(true);
    try {
      await updateMutation.mutateAsync({ server_host: serverHost });
      toast.success('Server host updated');
    } catch {
      toast.error('Failed to update server host');
    } finally {
      setServerHostLoading(false);
    }
  };

  return (
    <div>
      {/* Header */}
      <motion.div
        initial={{ opacity: 0, y: -8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.3 }}
        className="mb-6"
      >
        <h1 className="text-2xl font-bold text-base-content">Settings</h1>
        <p className="text-sm text-base-content/40 mt-1">Gateway configuration</p>
      </motion.div>

      {isLoading ? (
        <div className="flex items-center justify-center py-20">
          <span className="loading loading-spinner loading-lg text-base-content/20" />
        </div>
      ) : (
        <div className="max-w-2xl">
          {/* ── System Panel ── */}
          <motion.div
            initial={{ opacity: 0, y: 12 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.35, delay: 0.05 }}
            className="rounded-xl border border-base-300/60 bg-base-100 overflow-hidden"
          >
            {/* Panel header */}
            <div className="px-5 py-3 border-b border-base-300/60 bg-base-100/60">
              <span className="text-[10px] font-mono font-semibold uppercase tracking-[0.18em] text-base-content/25">
                SYSTEM CONFIGURATION
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
                    className="
                      flex-1 rounded-md border border-base-300 bg-base-200/50
                      px-3 py-1.5 text-sm text-base-content
                      placeholder:text-base-content/25
                      focus:outline-none focus:border-primary focus:ring-1 focus:ring-primary/30
                      transition-colors
                    "
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
          </motion.div>
        </div>
      )}
    </div>
  );
}
