import { useState } from 'react';
import { useSettings, useUpdateSettings } from '../hooks/useSettings';
import { changePassword } from '../api/auth';
import { Button } from '../components/ui/Button';
import { Toggle } from '../components/ui/Toggle';
import { Alert } from '../components/ui/Alert';
import { toast } from 'sonner';

type Tab = 'general' | 'password';

export default function Settings() {
  const [activeTab, setActiveTab] = useState<Tab>('general');
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
      <div className="mb-6"><h1 className="text-2xl font-bold">Settings</h1></div>

      <div className="tabs tabs-boxed mb-6">
        <button className={`tab ${activeTab === 'general' ? 'tab-active' : ''}`} onClick={() => setActiveTab('general')}>General</button>
        <button className={`tab ${activeTab === 'password' ? 'tab-active' : ''}`} onClick={() => setActiveTab('password')}>Password</button>
      </div>

      {activeTab === 'general' && (
        <>
          {isLoading ? (
            <div className="flex items-center justify-center py-12"><span className="loading loading-spinner loading-lg" /></div>
          ) : (
            <div className="max-w-lg bg-base-100 rounded-box p-5 shadow-sm mb-6">
              <div className="flex items-center justify-between py-3 border-b border-base-200">
                <div>
                  <span className="text-sm text-base-content/70">Allow Registration</span>
                  <p className="text-xs text-base-content/40">Allow new users to register</p>
                </div>
                <Toggle checked={settings?.allow_registration ?? false} onChange={(checked) => updateMutation.mutate({ allow_registration: checked })} />
              </div>

              <div className="flex items-center justify-between py-3 border-b border-base-200">
                <div>
                  <span className="text-sm text-base-content/70">Server Host</span>
                  <p className="text-xs text-base-content/40">Gateway base URL for proxy requests</p>
                </div>
                <input
                  type="text"
                  value={settings?.server_host ?? ''}
                  onChange={(e) => updateMutation.mutate({ server_host: e.target.value })}
                  placeholder="http://localhost:8080"
                  className="input input-bordered input-sm w-52 text-right font-mono text-xs text-base-content"
                />
              </div>

              <div className="flex items-center justify-between py-3 border-b border-base-200">
                <div>
                  <span className="text-sm text-base-content/70">Log Request Body</span>
                  <p className="text-xs text-base-content/40">Store request body in audit logs</p>
                </div>
                <Toggle checked={settings?.audit_log_request ?? true} onChange={(checked) => updateMutation.mutate({ audit_log_request: checked })} />
              </div>

              <div className="flex items-center justify-between py-3">
                <div>
                  <span className="text-sm text-base-content/70">Log Response Body</span>
                  <p className="text-xs text-base-content/40">Store response body in audit logs</p>
                </div>
                <Toggle checked={settings?.audit_log_response ?? true} onChange={(checked) => updateMutation.mutate({ audit_log_response: checked })} />
              </div>
            </div>
          )}
        </>
      )}

      {activeTab === 'password' && (
        <div className="max-w-lg bg-base-100 rounded-box p-5 shadow-sm">
          <h2 className="text-base font-semibold mb-4">Change Password</h2>

          {passwordStatus && (
            <Alert variant={passwordStatus.type === 'success' ? 'success' : 'error'} className="mb-4">
              {passwordStatus.message}
            </Alert>
          )}

          <form onSubmit={handleChangePassword} className="space-y-4">
            <div className="form-control">
              <label className="label"><span className="label-text">Current Password</span></label>
              <input type="password" value={currentPassword} onChange={(e) => setCurrentPassword(e.target.value)} required className="input input-bordered w-full" />
            </div>
            <div className="form-control">
              <label className="label"><span className="label-text">New Password</span></label>
              <input type="password" value={newPassword} onChange={(e) => setNewPassword(e.target.value)} required className="input input-bordered w-full" />
            </div>
            <div className="form-control">
              <label className="label"><span className="label-text">Confirm New Password</span></label>
              <input type="password" value={confirmPassword} onChange={(e) => setConfirmPassword(e.target.value)} required className="input input-bordered w-full" />
            </div>
            <Button variant="primary" loading={passwordLoading}>Change Password</Button>
          </form>
        </div>
      )}
    </div>
  );
}
