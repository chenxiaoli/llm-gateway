import { useState, useEffect } from 'react';
import { useSettings, useUpdateSettings } from '../hooks/useSettings';
import { changePassword } from '../api/auth';
import { Button } from '../components/ui/Button';
import { Alert } from '../components/ui/Alert';
import { Toggle } from '../components/ui/Toggle';
import { toast } from 'sonner';
import { motion } from 'framer-motion';

export default function Settings() {
  const { data: settings, isLoading } = useSettings();
  const updateMutation = useUpdateSettings();
  const [passwordLoading, setPasswordLoading] = useState(false);
  const [passwordStatus, setPasswordStatus] = useState<{ type: 'success' | 'error'; message: string } | null>(null);
  const [currentPassword, setCurrentPassword] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
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
        className="mb-6"
      >
        <h1 className="text-2xl font-bold text-base-content">Settings</h1>
        <p className="text-sm text-base-content/40 mt-1">Gateway configuration and account security</p>
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
                ACCOUNT SECURITY
              </span>
            </div>

            <div className="p-5 space-y-4">
              {passwordStatus && (
                <Alert
                  variant={passwordStatus.type === 'success' ? 'success' : 'error'}
                >
                  {passwordStatus.message}
                </Alert>
              )}

              <form onSubmit={handleChangePassword} className="space-y-3">
                <div className="space-y-1.5">
                  <label className="text-xs font-medium text-base-content/60">Current Password</label>
                  <input
                    type="password"
                    value={currentPassword}
                    onChange={(e) => setCurrentPassword(e.target.value)}
                    required
                    autoComplete="current-password"
                    className="
                      w-full h-9 rounded-md border border-base-300 bg-base-200/50
                      px-3 text-sm text-base-content
                      placeholder:text-base-content/25
                      focus:outline-none focus:border-primary focus:ring-1 focus:ring-primary/30
                      transition-colors
                    "
                  />
                </div>

                <div className="space-y-1.5">
                  <label className="text-xs font-medium text-base-content/60">New Password</label>
                  <input
                    type="password"
                    value={newPassword}
                    onChange={(e) => setNewPassword(e.target.value)}
                    required
                    autoComplete="new-password"
                    className="
                      w-full h-9 rounded-md border border-base-300 bg-base-200/50
                      px-3 text-sm text-base-content
                      placeholder:text-base-content/25
                      focus:outline-none focus:border-primary focus:ring-1 focus:ring-primary/30
                      transition-colors
                    "
                  />
                </div>

                <div className="space-y-1.5">
                  <label className="text-xs font-medium text-base-content/60">Confirm Password</label>
                  <input
                    type="password"
                    value={confirmPassword}
                    onChange={(e) => setConfirmPassword(e.target.value)}
                    required
                    autoComplete="new-password"
                    className="
                      w-full h-9 rounded-md border border-base-300 bg-base-200/50
                      px-3 text-sm text-base-content
                      placeholder:text-base-content/25
                      focus:outline-none focus:border-primary focus:ring-1 focus:ring-primary/30
                      transition-colors
                    "
                  />
                </div>

                <Button
                  type="submit"
                  variant="primary"
                  loading={passwordLoading}
                  className="w-full mt-1"
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
