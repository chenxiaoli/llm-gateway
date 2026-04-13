import { useState } from 'react';
import { useSettings, useUpdateSettings } from '../hooks/useSettings';
import { changePassword } from '../api/auth';
import { Button } from '../components/ui/Button';
import { Toggle } from '../components/ui/Toggle';
import { Alert } from '../components/ui/Alert';
import { toast } from 'sonner';

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
      <div className="mb-6"><h1 className="text-2xl font-bold">Settings</h1></div>

      {isLoading ? (
        <div className="flex items-center justify-center py-12"><span className="loading loading-spinner loading-lg" /></div>
      ) : (
        <div className="max-w-lg bg-base-100 rounded-box p-5 shadow-sm mb-6">
          <div className="flex items-center justify-between">
            <span className="text-sm text-base-content/70">Allow Registration</span>
            <Toggle checked={settings?.allow_registration ?? false} onChange={(checked) => updateMutation.mutate({ allow_registration: checked })} />
          </div>
        </div>
      )}

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
    </div>
  );
}
