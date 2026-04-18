import { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { ArrowLeft, Plus, Pencil, Trash2, RotateCcw } from 'lucide-react';
import { useProvider, useUpdateProvider, useDeleteProvider, useSyncModels } from '../hooks/useProviders';
import { useChannels, useCreateChannel, useUpdateChannel, useDeleteChannel } from '../hooks/useChannels';
import { useModels, useCreateModel, useUpdateModel, useDeleteModel } from '../hooks/useModels';
import { Button } from '../components/ui/Button';
import { Modal } from '../components/ui/Modal';
import { Badge } from '../components/ui/Badge';
import { Toggle } from '../components/ui/Toggle';
import { Select } from '../components/ui/Select';
import { ConfirmDialog } from '../components/ui/ConfirmDialog';
import type { Model, CreateModelRequest, UpdateModelRequest, Channel } from '../types';

// Billing types for model forms
const BILLING_TYPES = [
  { value: 'per_token', label: 'Per Token' },
  { value: 'per_request', label: 'Per Request' },
  { value: 'per_character', label: 'Per Character' },
  { value: 'tiered_token', label: 'Tiered Token' },
  { value: 'hybrid', label: 'Hybrid' },
] as const;

export default function ProviderDetail() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { data: provider, isLoading } = useProvider(id!);
  const updateMutation = useUpdateProvider();
  const deleteMutation = useDeleteProvider();
  const createModelMutation = useCreateModel(id!);
  const updateModelMutation = useUpdateModel(id!);
  const deleteModelMutation = useDeleteModel(id!);
  const createChannelMutation = useCreateChannel(id!);
  const updateChannelMutation = useUpdateChannel(id!);
  const deleteChannelMutation = useDeleteChannel(id!);
  const syncModelsMutation = useSyncModels(id!);

  const { data: channels } = useChannels(id!);
  const { data: models } = useModels(id!);

  const [provName, setProvName] = useState('');
  const [provBaseUrl, setProvBaseUrl] = useState('');
  const [provOpenaiUrl, setProvOpenaiUrl] = useState('');
  const [provAnthropicUrl, setProvAnthropicUrl] = useState('');
  const [provEnabled, setProvEnabled] = useState(false);

  const [modelModalOpen, setModelModalOpen] = useState(false);
  const [editingModel, setEditingModel] = useState<Model | null>(null);
  const [modelName, setModelName] = useState('');
  const [modelBillingType, setModelBillingType] = useState<string>('per_token');
  const [modelInputPrice, setModelInputPrice] = useState('');
  const [modelOutputPrice, setModelOutputPrice] = useState('');
  const [modelRequestPrice, setModelRequestPrice] = useState('');
  const [modelEnabled, setModelEnabled] = useState(false);

  const [channelModalOpen, setChannelModalOpen] = useState(false);
  const [editingChannel, setEditingChannel] = useState<Channel | null>(null);
  const [channelName, setChannelName] = useState('');
  const [channelApiKey, setChannelApiKey] = useState('');
  const [channelBaseUrl, setChannelBaseUrl] = useState('');
  const [channelPriority, setChannelPriority] = useState('0');
  const [channelEnabled, setChannelEnabled] = useState(false);

  const [activeTab, setActiveTab] = useState<'models' | 'channels'>('models');

  useEffect(() => {
    if (provider) {
      setProvName(provider.name);
      // endpoints is now already parsed as object
      let openaiUrl = '';
      let anthropicUrl = '';
      if (provider.endpoints) {
        openaiUrl = provider.endpoints.openai || '';
        anthropicUrl = provider.endpoints.anthropic || '';
      }
      setProvBaseUrl(provider.base_url ?? '');
      setProvOpenaiUrl(openaiUrl);
      setProvAnthropicUrl(anthropicUrl);
      setProvEnabled(provider.enabled);
    }
  }, [provider]);

  if (isLoading) return <div className="flex justify-center py-12"><span className="loading loading-spinner loading-lg" /></div>;
  if (!provider) return <div className="text-base-content/40">Provider not found</div>;

  const handleUpdateProvider = async (e: React.FormEvent) => {
    e.preventDefault();
    // Convert endpoints object back to JSON string for API
    const endpoints = JSON.stringify({
      openai: provOpenaiUrl || null,
      anthropic: provAnthropicUrl || null
    });
    await updateMutation.mutateAsync({ id: provider.id, input: { name: provName, base_url: provBaseUrl || null, endpoints, enabled: provEnabled } });
  };

  const handleDeleteProvider = async () => {
    await deleteMutation.mutateAsync(provider.id);
    navigate('/console/providers');
  };

  const resetModelForm = () => { setModelName(''); setModelBillingType('per_token'); setModelInputPrice(''); setModelOutputPrice(''); setModelRequestPrice(''); setModelEnabled(false); };
  const openAddModel = () => { setEditingModel(null); resetModelForm(); setModelModalOpen(true); };
  const openEditModel = (model: Model) => { setEditingModel(model); setModelName(model.name); setModelBillingType(model.billing_type); setModelInputPrice(String(model.input_price)); setModelOutputPrice(String(model.output_price)); setModelRequestPrice(String(model.request_price)); setModelEnabled(model.enabled); setModelModalOpen(true); };

  const handleSaveModel = async (e: React.FormEvent) => {
    e.preventDefault();
    if (editingModel) {
      const input: UpdateModelRequest = { billing_type: modelBillingType, input_price: Number(modelInputPrice) || 0, output_price: Number(modelOutputPrice) || 0, request_price: Number(modelRequestPrice) || 0, enabled: modelEnabled };
      await updateModelMutation.mutateAsync({ modelName: editingModel.name, input });
    } else {
      const input: CreateModelRequest = { name: modelName, billing_type: modelBillingType, input_price: Number(modelInputPrice) || 0, output_price: Number(modelOutputPrice) || 0, request_price: Number(modelRequestPrice) || 0 };
      await createModelMutation.mutateAsync(input);
    }
    setModelModalOpen(false);
  };

  const resetChannelForm = () => { setChannelName(''); setChannelApiKey(''); setChannelBaseUrl(''); setChannelPriority('0'); setChannelEnabled(false); };
  const openAddChannel = () => { setEditingChannel(null); resetChannelForm(); setChannelModalOpen(true); };
  const openEditChannel = (channel: Channel) => { setEditingChannel(channel); setChannelName(channel.name); setChannelApiKey(channel.api_key); setChannelBaseUrl(channel.base_url ?? ''); setChannelPriority(String(channel.priority)); setChannelEnabled(channel.enabled); setChannelModalOpen(true); };

  const handleSaveChannel = async (e: React.FormEvent) => {
    e.preventDefault();
    if (editingChannel) {
      await updateChannelMutation.mutateAsync({ id: editingChannel.id, input: { name: channelName, base_url: channelBaseUrl || null, priority: Number(channelPriority), enabled: channelEnabled } });
    } else {
      await createChannelMutation.mutateAsync({ provider_id: provider.id, name: channelName, api_key: channelApiKey, base_url: channelBaseUrl || null, priority: Number(channelPriority) });
    }
    setChannelModalOpen(false);
  };

  return (
    <div>
      <Button variant="ghost" icon={<ArrowLeft className="h-4 w-4" />} onClick={() => navigate('/console/providers')} className="mb-4">Back to Providers</Button>
      <div className="mb-6"><h1 className="text-2xl font-bold">Provider: {provider.name}</h1></div>

      <form onSubmit={handleUpdateProvider} className="mb-8 max-w-lg bg-base-100 rounded-box p-5 shadow-sm space-y-4">
        <div className="form-control"><label className="label"><span className="label-text">Name</span></label><input type="text" value={provName} onChange={(e) => setProvName(e.target.value)} required className="input input-bordered w-full" /></div>
        <div className="form-control"><label className="label"><span className="label-text">Base URL (Fallback)</span></label><input type="text" value={provBaseUrl} onChange={(e) => setProvBaseUrl(e.target.value)} placeholder="https://api.openai.com/v1" className="input input-bordered w-full" /></div>
        <div className="form-control"><label className="label"><span className="label-text">OpenAI Endpoint</span></label><input type="text" value={provOpenaiUrl} onChange={(e) => setProvOpenaiUrl(e.target.value)} placeholder="https://api.openai.com/v1" className="input input-bordered w-full" /></div>
        <div className="form-control"><label className="label"><span className="label-text">Anthropic Endpoint</span></label><input type="text" value={provAnthropicUrl} onChange={(e) => setProvAnthropicUrl(e.target.value)} placeholder="https://api.anthropic.com" className="input input-bordered w-full" /></div>
        <div className="flex items-center justify-between"><label className="label-text">Enabled</label><Toggle checked={provEnabled} onChange={setProvEnabled} /></div>
        <div className="flex gap-2">
          <Button variant="primary" type="submit" loading={updateMutation.isPending}>Save</Button>
          <ConfirmDialog title="Delete this provider and all its models?" onConfirm={handleDeleteProvider} okText="Delete"><Button variant="danger">Delete Provider</Button></ConfirmDialog>
        </div>
      </form>

      {/* Tabs */}
      <div className="tabs tabs-boxed mb-6">
        <button className={`tab ${activeTab === 'models' ? 'tab-active' : ''}`} onClick={() => setActiveTab('models')}>Models</button>
        <button className={`tab ${activeTab === 'channels' ? 'tab-active' : ''}`} onClick={() => setActiveTab('channels')}>Channels</button>
      </div>

      {/* Models Tab */}
      {activeTab === 'models' && (
        <div className="mb-8">
          <div className="flex items-center justify-between mb-3"><h2 className="text-base font-semibold">Models</h2><Button variant="ghost" icon={<RotateCcw className="h-4 w-4" />} onClick={() => syncModelsMutation.mutate()} loading={syncModelsMutation.isPending}>Sync Models</Button></div>
          <div className="overflow-x-auto bg-base-100 rounded-box shadow-sm">
            <table className="table table-sm">
              <thead><tr className="border-b border-base-300"><th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Name</th><th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Billing</th><th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Input ($/1M)</th><th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Output ($/1M)</th><th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Status</th><th className="text-xs font-semibold uppercase tracking-wider text-base-content/50 w-20">Actions</th></tr></thead>
              <tbody>
                {models?.map((model) => (
                  <tr key={model.id} className="border-b border-base-200 hover">
                    <td className="mono">{model.name}</td>
                    <td><Badge variant={model.billing_type === 'token' ? 'blue' : 'green'}>{model.billing_type}</Badge></td>
                    <td className="mono">{model.input_price.toFixed(2)}</td>
                    <td className="mono">{model.output_price.toFixed(2)}</td>
                    <td><Badge variant={model.enabled ? 'green' : 'red'}>{model.enabled ? 'Active' : 'Disabled'}</Badge></td>
                    <td>
                      <div className="flex items-center gap-1">
                        <button onClick={() => openEditModel(model)} className="btn btn-ghost btn-xs btn-circle" aria-label="Edit model">
                          <Pencil className="h-3.5 w-3.5" />
                        </button>
                        <ConfirmDialog title={`Delete model "${model.name}"?`} onConfirm={() => deleteModelMutation.mutateAsync(model.id)} okText="Delete">
                          <button className="btn btn-ghost btn-xs btn-circle text-error hover:text-error" aria-label="Delete model">
                            <Trash2 className="h-3.5 w-3.5" />
                          </button>
                        </ConfirmDialog>
                      </div>
                    </td>
                  </tr>
                ))}
                {(!models?.length) && (
                  <tr>
                    <td colSpan={6} className="text-center py-12">
                      <div className="flex flex-col items-center gap-2">
                        <span className="text-base-content/25 text-sm">No models configured</span>
                        <button onClick={openAddModel} className="link link-primary text-sm">Add your first model</button>
                      </div>
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {/* Channels Tab */}
      {activeTab === 'channels' && (
        <div className="mb-8">
          <div className="flex items-center justify-between mb-3"><h2 className="text-base font-semibold">Channels</h2><Button icon={<Plus className="h-4 w-4" />} onClick={openAddChannel}>Add Channel</Button></div>
          <div className="overflow-x-auto bg-base-100 rounded-box shadow-sm">
            <table className="table table-sm">
              <thead><tr className="border-b border-base-300"><th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Name</th><th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Base URL</th><th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Priority</th><th className="text-xs font-semibold uppercase tracking-wider text-base-content/50">Status</th><th className="text-xs font-semibold uppercase tracking-wider text-base-content/50 w-20">Actions</th></tr></thead>
              <tbody>
                {channels?.map((channel) => (
                  <tr key={channel.id} className="border-b border-base-200 hover">
                    <td>{channel.name}</td>
                    <td className="mono">{channel.base_url ? channel.base_url : <span className="text-base-content/40">Default</span>}</td>
                    <td className="mono">{channel.priority}</td>
                    <td><Badge variant={channel.enabled ? 'green' : 'red'}>{channel.enabled ? 'Active' : 'Disabled'}</Badge></td>
                    <td>
                      <div className="flex items-center gap-1">
                        <button onClick={() => openEditChannel(channel)} className="btn btn-ghost btn-xs btn-circle" aria-label="Edit channel">
                          <Pencil className="h-3.5 w-3.5" />
                        </button>
                        <ConfirmDialog title={`Delete channel "${channel.name}"?`} onConfirm={() => deleteChannelMutation.mutateAsync(channel.id)} okText="Delete">
                          <button className="btn btn-ghost btn-xs btn-circle text-error hover:text-error" aria-label="Delete channel">
                            <Trash2 className="h-3.5 w-3.5" />
                          </button>
                        </ConfirmDialog>
                      </div>
                    </td>
                  </tr>
                ))}
                {(!channels?.length) && (
                  <tr>
                    <td colSpan={5} className="text-center py-12">
                      <div className="flex flex-col items-center gap-2">
                        <span className="text-base-content/25 text-sm">No channels configured</span>
                        <button onClick={openAddChannel} className="link link-primary text-sm">Add your first channel</button>
                      </div>
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </div>
      )}

      <Modal open={modelModalOpen} onClose={() => setModelModalOpen(false)} title={editingModel ? `Edit Model: ${editingModel.name}` : 'Add Model'}>
        <form onSubmit={handleSaveModel} className="space-y-4">
          {!editingModel && (<div className="form-control"><label className="label"><span className="label-text">Model Name</span></label><input type="text" value={modelName} onChange={(e) => setModelName(e.target.value)} placeholder="e.g., gpt-4o" required className="input input-bordered w-full" /></div>)}
          <div className="form-control"><label className="label"><span className="label-text">Billing Type</span></label><Select value={modelBillingType} onChange={(v) => setModelBillingType(v as string)} options={[...BILLING_TYPES]} /></div>
          <div className="grid grid-cols-3 gap-4">
            <div className="form-control"><label className="label"><span className="label-text">Input ($/1M)</span></label><input type="number" value={modelInputPrice} onChange={(e) => setModelInputPrice(e.target.value)} min={0} step={0.01} className="input input-bordered w-full" /></div>
            <div className="form-control"><label className="label"><span className="label-text">Output ($/1M)</span></label><input type="number" value={modelOutputPrice} onChange={(e) => setModelOutputPrice(e.target.value)} min={0} step={0.01} className="input input-bordered w-full" /></div>
            <div className="form-control"><label className="label"><span className="label-text">Request ($/req)</span></label><input type="number" value={modelRequestPrice} onChange={(e) => setModelRequestPrice(e.target.value)} min={0} step={0.01} className="input input-bordered w-full" /></div>
          </div>
          {editingModel && (<div className="flex items-center justify-between"><label className="label-text">Enabled</label><Toggle checked={modelEnabled} onChange={setModelEnabled} /></div>)}
          <Button variant="primary" type="submit">{editingModel ? 'Update' : 'Create'}</Button>
        </form>
      </Modal>

      <Modal open={channelModalOpen} onClose={() => setChannelModalOpen(false)} title={editingChannel ? `Edit Channel: ${editingChannel.name}` : 'Add Channel'}>
        <form onSubmit={handleSaveChannel} className="space-y-4">
          <div className="form-control"><label className="label"><span className="label-text">Name</span></label><input type="text" value={channelName} onChange={(e) => setChannelName(e.target.value)} placeholder="e.g., primary" required className="input input-bordered w-full" /></div>
          <div className="form-control"><label className="label"><span className="label-text">API Key</span></label><input type="password" value={channelApiKey} onChange={(e) => setChannelApiKey(e.target.value)} placeholder="Upstream API key" required className="input input-bordered w-full" /></div>
          <div className="form-control"><label className="label"><span className="label-text">Base URL</span></label><input type="text" value={channelBaseUrl} onChange={(e) => setChannelBaseUrl(e.target.value)} placeholder="Leave empty to use provider default" className="input input-bordered w-full" /></div>
          <div className="form-control"><label className="label"><span className="label-text">Priority</span></label><input type="number" value={channelPriority} onChange={(e) => setChannelPriority(e.target.value)} min={0} className="input input-bordered w-full" /></div>
          {editingChannel && (<div className="flex items-center justify-between"><label className="label-text">Enabled</label><Toggle checked={channelEnabled} onChange={setChannelEnabled} /></div>)}
          <Button variant="primary" type="submit" loading={createChannelMutation.isPending || updateChannelMutation.isPending}>{editingChannel ? 'Update' : 'Create'}</Button>
        </form>
      </Modal>
    </div>
  );
}
