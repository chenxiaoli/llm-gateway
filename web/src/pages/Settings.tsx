import { useState } from 'react';
import { useSettings, useUpdateSettings } from '../hooks/useSettings';
import { changePassword } from '../api/auth';
import { Button } from '../components/ui/Button';
import { Alert } from '../components/ui/Alert';
import { toast } from 'sonner';
import { motion } from 'framer-motion';

function StatusDot({ active }: { active: boolean }) {
  return (
    <span className="relative flex h-2 w-2 shrink-0">
      {active && (
        <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-success opacity-75" />
      )}
      <span className={`relative inline-flex rounded-full h-2 w-2 ${active ? 'bg-success' : 'bg-base-content/20'}`} />
    </span>
  );
}

function SettingRow({
  label,
  description,
  children,
}: {
  label: string;
  description: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-center justify-between gap-4 py-3 border-b border-base-200/60 last:border-0">
      <div className="min-w-0">
        <div className="flex items-center gap-2">
          <StatusDot active={false} />
          <span className="text-[13px] font-mono font-semibold text-base-content/70 tracking-tight">{label}</span>
        </div>
        <p className="text-[11px] font-mono text-base-content/30 mt-0.5 ml-4 leading-relaxed">{description}</p>
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

function ToggleRow({
  label,
  description,
  checked,
  onChange,
}: {
  label: string;
  description: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <SettingRow label={label} description={description}>
      <button
        role="switch"
        aria-checked={checked}
        onClick={() => onChange(!checked)}
        className={`
          relative inline-flex h-5 w-9 items-center rounded-full transition-all duration-200 focus:outline-none focus-visible:ring-2 focus-visible:ring-primary/50 focus-visible:ring-offset-1 focus-visible:ring-offset-base-100
          ${checked ? 'bg-success/20' : 'bg-base-300/60'}
        `}
      >
        <span
          className={`
            inline-block h-3.5 w-3.5 transform rounded-full bg-base-100 shadow-sm transition-all duration-200
            ${checked ? 'translate-x-[18px]' : 'translate-x-1'}
          `}
        />
        <span
          className={`absolute inset-0 flex items-center transition-all duration-200 ${checked ? 'justify-end pr-1' : 'justify-start pl-1'}`}
        >
          <span className={`h-1 w-1 rounded-full ${checked ? 'bg-success' : 'bg-base-content/30'}`} />
        </span>
      </button>
    </SettingRow>
  );
}

export default function Settings() {
  const { data: settings, isLoading } = useSettings();
  const updateMutation = useUpdateSettings();
  const [passwordLoading, setPasswordLoading] = useState(false);
  const [passwordStatus, setPasswordStatus] = useState<{ type: 'success' | 'error'; message: string } | null>(null);
  const [currentPassword, setCurrentPassword] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');

  const handleChangePassword = async (e: React.FormEvent) => {
    e.preventDefault();
    if (newPassword !== confirmPassword) { toast.error('Passwords do not match'); return; }
    if (!currentPassword || !newPassword) return;
    setPasswordLoading(true);
    setPasswordStatus(null);
    try {
      await changePassword({ current_password: currentPassword, new_password: newPassword });
      setPasswordStatus({ type: 'success', message: 'Password changed successfully' });
      setCurrentPassword(''); setNewPassword(''); setConfirmPassword('');
    } catch {
      setPasswordStatus({ type: 'error', message: 'Failed to change password' });
    } finally {
      setPasswordLoading(false);
    }
  };

  return (
    <div>
      {/* Header */}
      <motion.div
        initial={{ opacity: 0, y: -8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.3 }}
        className="mb-8"
      >
        <h1 className="text-2xl font-bold font-mono tracking-tight text-base-content">Settings</h1>
        <p className="text-sm font-mono text-base-content/30 mt-1">Gateway configuration and account security</p>
      </motion.div>

      {isLoading ? (
        <div className="flex items-center justify-center py-20">
          <span className="loading loading-spinner loading-lg text-base-content/20" />
        </div>
      ) : (
        <div className="grid grid-cols-1 lg:grid-cols-[1fr_360px] gap-5">
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
                // SYSTEM CONFIGURATION
              </span>
            </div>

            <div className="px-5 divide-y divide-base-200/50">
              <ToggleRow
                label="Allow Registration"
                description="Permit new users to register on the login page"
                checked={settings?.allow_registration ?? false}
                onChange={(v) => updateMutation.mutate({ allow_registration: v })}
              />
              <ToggleRow
                label="Log Request Body"
                description="Capture and store upstream request payloads in audit logs"
                checked={settings?.audit_log_request ?? true}
                onChange={(v) => updateMutation.mutate({ audit_log_request: v })}
              />
              <ToggleRow
                label="Log Response Body"
                description="Capture and store upstream response payloads in audit logs"
                checked={settings?.audit_log_response ?? true}
                onChange={(v) => updateMutation.mutate({ audit_log_response: v })}
              />

              {/* Server Host — text input */}
              <div className="flex items-center justify-between gap-4 py-3">
                <div className="min-w-0">
                  <div className="flex items-center gap-2">
                    <StatusDot active={!!settings?.server_host} />
                    <span className="text-[13px] font-mono font-semibold text-base-content/70 tracking-tight">Server Host</span>
                  </div>
                  <p className="text-[11px] font-mono text-base-content/30 mt-0.5 ml-4 leading-relaxed">
                    Gateway base URL used by the proxy endpoint
                  </p>
                </div>
                <input
                  type="text"
                  value={settings?.server_host ?? ''}
                  onChange={(e) => updateMutation.mutate({ server_host: e.target.value })}
                  placeholder="http://localhost:8080"
                  spellCheck={false}
                  className="
                    w-56 h-8 rounded-md border border-base-300/80 bg-base-200/40
                    px-3 text-[12px] font-mono text-base-content/80
                    placeholder:text-base-content/20
                    focus:outline-none focus:border-primary/50 focus:ring-1 focus:ring-primary/20
                    transition-colors duration-150
                  "
                />
              </div>
            </div>
          </motion.div>

          {/* ── Security Panel ── */}
          <motion.div
            initial={{ opacity: 0, y: 12 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.35, delay: 0.1 }}
            className="rounded-xl border border-base-300/60 bg-base-100 overflow-hidden"
          >
            {/* Panel header */}
            <div className="px-5 py-3 border-b border-base-300/60 bg-base-100/60">
              <span className="text-[10px] font-mono font-semibold uppercase tracking-[0.18em] text-base-content/25">
                // ACCOUNT SECURITY
              </span>
            </div>

            <div className="p-5">
              {passwordStatus && (
                <Alert
                  variant={passwordStatus.type === 'success' ? 'success' : 'error'}
                  className="mb-4 font-mono text-xs"
                >
                  {passwordStatus.message}
                </Alert>
              )}

              <form onSubmit={handleChangePassword} className="space-y-3">
                <div className="space-y-1">
                  <label className="text-[10px] font-mono font-semibold uppercase tracking-widest text-base-content/35">
                    Current Password
                  </label>
                  <input
                    type="password"
                    value={currentPassword}
                    onChange={(e) => setCurrentPassword(e.target.value)}
                    required
                    autoComplete="current-password"
                    className="
                      w-full h-9 rounded-md border border-base-300/80 bg-base-200/40
                      px-3 text-[13px] font-mono text-base-content
                      placeholder:text-base-content/20
                      focus:outline-none focus:border-primary/50 focus:ring-1 focus:ring-primary/20
                      transition-colors duration-150
                    "
                  />
                </div>

                <div className="space-y-1">
                  <label className="text-[10px] font-mono font-semibold uppercase tracking-widest text-base-content/35">
                    New Password
                  </label>
                  <input
                    type="password"
                    value={newPassword}
                    onChange={(e) => setNewPassword(e.target.value)}
                    required
                    autoComplete="new-password"
                    className="
                      w-full h-9 rounded-md border border-base-300/80 bg-base-200/40
                      px-3 text-[13px] font-mono text-base-content
                      placeholder:text-base-content/20
                      focus:outline-none focus:border-primary/50 focus:ring-1 focus:ring-primary/20
                      transition-colors duration-150
                    "
                  />
                </div>

                <div className="space-y-1">
                  <label className="text-[10px] font-mono font-semibold uppercase tracking-widest text-base-content/35">
                    Confirm Password
                  </label>
                  <input
                    type="password"
                    value={confirmPassword}
                    onChange={(e) => setConfirmPassword(e.target.value)}
                    required
                    autoComplete="new-password"
                    className="
                      w-full h-9 rounded-md border border-base-300/80 bg-base-200/40
                      px-3 text-[13px] font-mono text-base-content
                      placeholder:text-base-content/20
                      focus:outline-none focus:border-primary/50 focus:ring-1 focus:ring-primary/20
                      transition-colors duration-150
                    "
                  />
                </div>

                <Button
                  type="submit"
                  variant="primary"
                  loading={passwordLoading}
                  className="w-full font-mono text-xs mt-2"
                >
                  Update Password
                </Button>
              </form>
            </div>
          </motion.div>
        </div>
      )}
    </div>
  );
}
