import { useState } from 'react';
import { useSettings, useUpdateSettings } from '../hooks/useSettings';
import { changePassword } from '../api/auth';
import { useProviders, useCreateProvider, useUpdateProvider, useDeleteProvider } from '../hooks/useProviders';
import { useModels, useCreateModel, useCreateGlobalModel } from '../hooks/useModels';
import { getSeedData, createProvider } from '../api/providers';
import { Button } from '../components/ui/Button';
import { Toggle } from '../components/ui/Toggle';
import { Alert } from '../components/ui/Alert';
import { Drawer } from '../components/ui/Drawer';
import { Modal } from '../components/ui/Modal';
import { EndpointsEditor } from '../components/ui/EndpointsEditor';
import { ConfirmDialog } from '../components/ui/ConfirmDialog';
import { Checkbox } from '../components/ui/Checkbox';
import { toast } from 'sonner';
import { Plus, Pencil, Trash2, Upload, Check } from 'lucide-react';
import type { Provider } from '../types';

type Tab = 'general' | 'providers' | 'password';

export default function Settings() {
  const [activeTab, setActiveTab] = useState<Tab>('general');
  const { data: settings, isLoading } = useSettings();
  const updateMutation = useUpdateSettings();
  const [passwordLoading, setPasswordLoading] = useState(false);
  const [passwordStatus, setPasswordStatus] = useState<{ type: 'success' | 'error'; message: string } | null>(null);
  const [currentPassword, setCurrentPassword] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');

  const { data: providers } = useProviders();
  const createProviderMutation = useCreateProvider();
  const createModelMutation = useCreateGlobalModel();
  const updateProviderMutation = useUpdateProvider();
  const deleteProviderMutation = useDeleteProvider();

  const [providerModalOpen, setProviderModalOpen] = useState(false);
  const [importModalOpen, setImportModalOpen] = useState(false);
  const [seedData, setSeedData] = useState<{ providers: Array<{ name: string; base_url?: string; endpoints?: Record<string, string>; enabled?: boolean; selected?: boolean }>; models: Array<{ provider: string; name: string; billing_type?: string; input_price?: number; output_price?: number; selected?: boolean }> } | null>(null);
  const [seedLoading, setSeedLoading] = useState(false);
  const [editingProvider, setEditingProvider] = useState<Provider | null>(null);
  const [provName, setProvName] = useState('');
  const [provBaseUrl, setProvBaseUrl] = useState('');
  const [provEndpoints, setProvEndpoints] = useState<Record<string, string>>({});
  const [provEnabled, setProvEnabled] = useState(true);

  const openImportModal = async () => {
    setSeedLoading(true);
    setImportModalOpen(true);
    try {
      const data = await getSeedData();
      // Initialize selected state
      setSeedData({
        providers: data.providers.map(p => ({ ...p, selected: true })),
        models: data.models.map(m => ({ ...m, selected: true }))
      });
    } catch (err) {
      toast.error('Failed to load seed data');
    }
    setSeedLoading(false);
  };

  const handleImportSeed = async () => {
    if (!seedData) return;
    const providerMap: Record<string, string> = {};
    // Import selected providers first
    for (const p of seedData.providers.filter(p => p.selected)) {
      const created = await createProviderMutation.mutateAsync({
        name: p.name,
        base_url: p.base_url || null,
        endpoints: p.endpoints ? JSON.stringify(p.endpoints) : null,
      });
      providerMap[p.name] = created.id;
    }
    // Import selected models
    for (const m of seedData.models.filter(m => m.selected)) {
      const providerId = providerMap[m.provider];
      if (providerId) {
        await createModelMutation.mutateAsync({
          provider_id: providerId,
          name: m.name,
          billing_type: m.billing_type || 'per_token',
          input_price: m.input_price,
          output_price: m.output_price,
        });
      }
    }
    toast.success('Import completed');
    setImportModalOpen(false);
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

  const openAddProvider = () => { setEditingProvider(null); setProvName(''); setProvBaseUrl(''); setProvEndpoints({}); setProvEnabled(true); setProviderModalOpen(true); };
  const openEditProvider = (p: Provider) => {
    let parsedEndpoints: Record<string, string> = {};
    if (p.endpoints) {
      try {
        parsedEndpoints = JSON.parse(p.endpoints);
      } catch {}
    }
    setEditingProvider(p);
    setProvName(p.name);
    setProvBaseUrl(p.base_url ?? '');
    setProvEndpoints(parsedEndpoints);
    setProvEnabled(p.enabled);
    setProviderModalOpen(true);
  };
  const handleSaveProvider = async (e: React.FormEvent) => {
    e.preventDefault();
    const endpoints: Record<string, string | null> = {};
    Object.entries(provEndpoints).forEach(([key, value]) => {
      endpoints[key] = value || null;
    });

    if (editingProvider) {
      await updateProviderMutation.mutateAsync({ id: editingProvider.id, input: { name: provName, base_url: provBaseUrl || null, endpoints: Object.keys(endpoints).length > 0 ? JSON.stringify(endpoints) : null, enabled: provEnabled } });
    } else {
      await createProviderMutation.mutateAsync({ name: provName, base_url: provBaseUrl || null, endpoints: Object.keys(endpoints).length > 0 ? JSON.stringify(endpoints) : null });
    }
    setProviderModalOpen(false);
  };
  const handleDeleteProvider = async (id: string) => { await deleteProviderMutation.mutateAsync(id); };

  return (
    <div>
      <div className="mb-6"><h1 className="text-2xl font-bold">Settings</h1></div>

      <div className="tabs tabs-boxed mb-6">
        <button className={`tab ${activeTab === 'general' ? 'tab-active' : ''}`} onClick={() => setActiveTab('general')}>General</button>
        <button className={`tab ${activeTab === 'providers' ? 'tab-active' : ''}`} onClick={() => setActiveTab('providers')}>Providers</button>
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

      {activeTab === 'providers' && (
        <div className="mb-6 flex gap-2 justify-end">
          <Button variant="secondary" icon={<Upload className="h-4 w-4" />} onClick={openImportModal}>Import</Button>
          <Button icon={<Plus className="h-4 w-4" />} onClick={openAddProvider}>Add Provider</Button>
        </div>
      )}

      {/* Import Modal */}
      <Modal open={importModalOpen} onClose={() => setImportModalOpen(false)} title="Import Providers & Models">
        {seedLoading ? (
          <div className="p-4">Loading...</div>
        ) : seedData ? (
          <div className="space-y-4 max-h-[70vh] overflow-y-auto">
            <div className="space-y-2">
              <h4 className="font-medium">Providers ({seedData.providers.filter(p => p.selected).length} selected)</h4>
              {seedData.providers.map(p => (
                <label key={p.name} className="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" checked={p.selected} onChange={(e) => {
                    setSeedData({
                      ...seedData,
                      providers: seedData.providers.map(np => np.name === p.name ? { ...np, selected: e.target.checked } : np)
                    });
                  }} />
                  <span>{p.name}</span>
                </label>
              ))}
            </div>
            <div className="space-y-2">
              <h4 className="font-medium">Models ({seedData.models.filter(m => m.selected).length} selected)</h4>
              {seedData.models.map((m, i) => (
                <label key={`${m.provider}-${m.name}-${i}`} className="flex items-center gap-2 cursor-pointer">
                  <input type="checkbox" checked={m.selected} onChange={(e) => {
                    setSeedData({
                      ...seedData,
                      models: seedData.models.map((nm, ni) => ni === i ? { ...nm, selected: e.target.checked } : nm)
                    });
                  }} />
                  <span>{m.name}</span>
                  <span className="text-xs text-base-content/40">({m.provider})</span>
                </label>
              ))}
            </div>
            <div className="flex gap-2 justify-end pt-2 border-t">
              <Button variant="secondary" onClick={() => setImportModalOpen(false)}>Cancel</Button>
              <Button variant="primary" onClick={handleImportSeed}>Import Selected</Button>
            </div>
          </div>
        ) : (
          <div className="p-4">No seed data available</div>
        )}
      </Modal>

      {activeTab === 'providers' && providers?.map((provider) => (
        <div key={provider.id} className="max-w-lg bg-base-100 rounded-box p-5 shadow-sm mb-4">
          <div className="flex items-center justify-between mb-4">
            <div>
              <h3 className="font-semibold">{provider.name}</h3>
              <p className="text-xs text-base-content/40">{provider.base_url}</p>
            </div>
            <div className="flex items-center gap-2">
              <button onClick={() => openEditProvider(provider)} className="btn btn-ghost btn-sm btn-circle"><Pencil className="h-4 w-4" /></button>
              <ConfirmDialog title={`Delete provider "${provider.name}"?`} onConfirm={() => handleDeleteProvider(provider.id)} okText="Delete"><button className="btn btn-ghost btn-sm btn-circle text-error"><Trash2 className="h-4 w-4" /></button></ConfirmDialog>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <span className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-xs font-medium ${provider.enabled ? 'bg-success/10 text-success' : 'bg-base-300/50 text-base-content/40'}`}>
              <span className={`w-1.5 h-1.5 rounded-full ${provider.enabled ? 'bg-success' : 'bg-base-content/30'}`} />
              {provider.enabled ? 'Active' : 'Disabled'}
            </span>
          </div>
        </div>
      ))}

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

      <Drawer
        open={providerModalOpen}
        onClose={() => setProviderModalOpen(false)}
        title={editingProvider ? 'Edit Provider' : 'Add Provider'}
      >
        <form onSubmit={handleSaveProvider} className="space-y-4">
          <div className="form-control">
            <label className="label"><span className="label-text">Name</span></label>
            <input type="text" value={provName} onChange={(e) => setProvName(e.target.value)} required className="input input-bordered w-full" />
          </div>
          <div className="form-control">
            <label className="label"><span className="label-text">Base URL (Fallback)</span></label>
            <input type="text" value={provBaseUrl} onChange={(e) => setProvBaseUrl(e.target.value)} placeholder="https://api.openai.com/v1" className="input input-bordered w-full" />
          </div>
          <div className="form-control">
            <label className="label"><span className="label-text">Endpoints</span></label>
            <EndpointsEditor value={provEndpoints} onChange={setProvEndpoints} />
          </div>
          {editingProvider && (
            <div className="flex items-center justify-between">
              <label className="label-text">Enabled</label>
              <Toggle checked={provEnabled} onChange={setProvEnabled} />
            </div>
          )}
          <Button variant="primary" type="submit">{editingProvider ? 'Update' : 'Create'}</Button>
        </form>
      </Drawer>
    </div>
  );
}
