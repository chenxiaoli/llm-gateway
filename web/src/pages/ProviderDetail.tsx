import { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { ArrowLeft, Plus, Pencil, Trash2, RotateCcw } from 'lucide-react';
import { useProvider, useUpdateProvider, useDeleteProvider, useSyncModels } from '../hooks/useProviders';
import { useChannels, useCreateChannel, useUpdateChannel, useDeleteChannel } from '../hooks/useChannels';
import { useChannelModels, useCreateChannelModel, useUpdateChannelModel, useDeleteChannelModel } from '../hooks/useChannelModels';
import { usePricingPolicies } from '../hooks/usePricingPolicies';
import { Button } from '../components/ui/Button';
import { Modal } from '../components/ui/Modal';
import { Badge } from '../components/ui/Badge';
import { Toggle } from '../components/ui/Toggle';
import { Select } from '../components/ui/Select';
import { ConfirmDialog } from '../components/ui/ConfirmDialog';
import type { ChannelModel, CreateChannelModelRequest, UpdateChannelModelRequest, Channel } from '../types';

export default function ProviderDetail() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { data: provider, isLoading } = useProvider(id!);
  const updateMutation = useUpdateProvider();
  const deleteMutation = useDeleteProvider();
  const createChannelMutation = useCreateChannel(id!);
  const updateChannelMutation = useUpdateChannel(id!);
  const deleteChannelMutation = useDeleteChannel(id!);
  const syncModelsMutation = useSyncModels(id!);

  const { data: channels } = useChannels(id!);
  const { data: channelModels } = useChannelModels(id!);
  const createModelMutation = useCreateChannelModel(id!);
  const updateModelMutation = useUpdateChannelModel(id!);
  const deleteModelMutation = useDeleteChannelModel(id!);

  const { data: policies } = usePricingPolicies();

  const [provName, setProvName] = useState('');
  const [provOpenaiUrl, setProvOpenaiUrl] = useState('');
  const [provAnthropicUrl, setProvAnthropicUrl] = useState('');
  const [provEnabled, setProvEnabled] = useState(false);
  const [provProxyUrl, setProvProxyUrl] = useState('');

  const [modelModalOpen, setModelModalOpen] = useState(false);
  const [editingChannelModel, setEditingChannelModel] = useState<ChannelModel | null>(null);
  const [modelId, setModelId] = useState('');
  const [upstreamModelName, setUpstreamModelName] = useState('');
  const [priorityOverride, setPriorityOverride] = useState('');
  const [pricingPolicyId, setPricingPolicyId] = useState('');
  const [markupRatio, setMarkupRatio] = useState('1.0');
  const [modelEnabled, setModelEnabled] = useState(false);

  const [channelModalOpen, setChannelModalOpen] = useState(false);
  const [editingChannel, setEditingChannel] = useState<Channel | null>(null);
  const [channelName, setChannelName] = useState('');
  const [channelApiKey, setChannelApiKey] = useState('');
  const [channelPriority, setChannelPriority] = useState('0');
  const [channelEnabled, setChannelEnabled] = useState(false);

  const [activeTab, setActiveTab] = useState<'models' | 'channels'>('models');

  useEffect(() => {
    if (provider) {
      setProvName(provider.name);
      let openaiUrl = '';
      let anthropicUrl = '';
      if (provider.endpoints) {
        openaiUrl = provider.endpoints.openai || '';
        anthropicUrl = provider.endpoints.anthropic || '';
      }
      setProvOpenaiUrl(openaiUrl);
      setProvAnthropicUrl(anthropicUrl);
      setProvEnabled(provider.enabled);
      setProvProxyUrl(provider.proxy_url || '');
    }
  }, [provider]);

  if (isLoading) return <div className="flex justify-center py-12"><span className="loading loading-spinner loading-lg" /></div>;
  if (!provider) return <div className="text-base-content/40">Provider not found</div>;

  const handleUpdateProvider = async (e: React.FormEvent) => {
    e.preventDefault();
    const endpoints: Record<string, string | null> = {
      openai: provOpenaiUrl || null,
      anthropic: provAnthropicUrl || null
    };
    await updateMutation.mutateAsync({ id: provider.id, input: { name: provName, endpoints, proxy_url: provProxyUrl || null, enabled: provEnabled } });
  };

  const handleDeleteProvider = async () => {
    await deleteMutation.mutateAsync(provider.id);
    navigate('/console/providers');
  };

  const resetModelForm = () => {
    setModelId(''); setUpstreamModelName(''); setPriorityOverride('');
    setPricingPolicyId(''); setMarkupRatio('1.0'); setModelEnabled(false);
  };
  const openAddModel = () => { setEditingChannelModel(null); resetModelForm(); setModelModalOpen(true); };
  const openEditModel = (cm: ChannelModel) => {
    setEditingChannelModel(cm); setModelId(cm.model_id);
    setUpstreamModelName(cm.upstream_model_name ?? '');
    setPriorityOverride(cm.priority_override != null ? String(cm.priority_override) : '');
    setPricingPolicyId(cm.pricing_policy_id ?? '');
    setMarkupRatio(String(cm.markup_ratio));
    setModelEnabled(cm.enabled);
    setModelModalOpen(true);
  };

  const handleSaveModel = async (e: React.FormEvent) => {
    e.preventDefault();
    if (editingChannelModel) {
      const input: UpdateChannelModelRequest = {
        upstream_model_name: upstreamModelName || undefined,
        priority_override: priorityOverride ? Number(priorityOverride) : undefined,
        pricing_policy_id: pricingPolicyId || undefined,
        markup_ratio: Number(markupRatio),
        enabled: modelEnabled,
      };
      await updateModelMutation.mutateAsync({ id: editingChannelModel.id, input });
    } else {
      const input: CreateChannelModelRequest = {
        model_id: modelId,
        upstream_model_name: upstreamModelName || undefined,
        priority_override: priorityOverride ? Number(priorityOverride) : undefined,
        pricing_policy_id: pricingPolicyId || undefined,
        markup_ratio: Number(markupRatio),
        enabled: true,
      };
      await createModelMutation.mutateAsync(input);
    }
    setModelModalOpen(false);
  };

  const resetChannelForm = () => { setChannelName(''); setChannelApiKey(''); setChannelPriority('0'); setChannelEnabled(false); };
  const openAddChannel = () => { setEditingChannel(null); resetChannelForm(); setChannelModalOpen(true); };
  const openEditChannel = (channel: Channel) => { setEditingChannel(channel); setChannelName(channel.name); setChannelApiKey(channel.api_key); setChannelPriority(String(channel.priority)); setChannelEnabled(channel.enabled); setChannelModalOpen(true); };

  const handleSaveChannel = async (e: React.FormEvent) => {
    e.preventDefault();
    if (editingChannel) {
      await updateChannelMutation.mutateAsync({ id: editingChannel.id, input: { name: channelName, priority: Number(channelPriority), enabled: channelEnabled } });
    } else {
      await createChannelMutation.mutateAsync({ provider_id: provider.id, name: channelName, api_key: channelApiKey, priority: Number(channelPriority) });
    }
    setChannelModalOpen(false);
  };

  return (
    <div>
      <Button variant="ghost" icon={<ArrowLeft className="h-4 w-4" />} onClick={() => navigate('/console/providers')} className="mb-4">Back to Providers</Button>
      <div className="mb-6"><h1 className="text-4xl font-bold">Provider: {provider.name}</h1></div>

      <form onSubmit={handleUpdateProvider} className="mb-8 max-w-lg bg-base-100 rounded-box p-5 shadow-sm space-y-4">
        <div className="form-control"><label className="label"><span className="label-text">Name</span></label><input type="text" value={provName} onChange={(e) => setProvName(e.target.value)} required className="input input-bordered w-full" /></div>
        <div className="form-control"><label className="label"><span className="label-text">OpenAI Endpoint</span></label><input type="text" value={provOpenaiUrl} onChange={(e) => setProvOpenaiUrl(e.target.value)} placeholder="https://api.openai.com/v1" className="input input-bordered w-full" /></div>
        <div className="form-control"><label className="label"><span className="label-text">Anthropic Endpoint</span></label><input type="text" value={provAnthropicUrl} onChange={(e) => setProvAnthropicUrl(e.target.value)} placeholder="https://api.anthropic.com" className="input input-bordered w-full" /></div>
        <div className="form-control"><label className="label"><span className="label-text">Proxy URL</span></label><input type="text" value={provProxyUrl} onChange={(e) => setProvProxyUrl(e.target.value)} placeholder="http://proxy:8080" className="input input-bordered w-full" /></div>
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
          <div className="flex items-center justify-between mb-3">
            <h2 className="text-lg font-semibold">Channel Models</h2>
            <div className="flex gap-2">
              <Button variant="ghost" icon={<RotateCcw className="h-4 w-4" />} onClick={() => syncModelsMutation.mutate()} loading={syncModelsMutation.isPending}>Sync Models</Button>
              <Button variant="ghost" icon={<Plus className="h-4 w-4" />} onClick={openAddModel}>Add Model</Button>
            </div>
          </div>
          <div className="overflow-x-auto bg-base-100 rounded-box shadow-sm">
            <table className="table table-sm">
              <thead><tr className="border-b border-base-300">
                <th className="text-base font-semibold uppercase tracking-wider text-base-content/50">Model ID</th>
                <th className="text-base font-semibold uppercase tracking-wider text-base-content/50">Upstream Name</th>
                <th className="text-base font-semibold uppercase tracking-wider text-base-content/50">Priority</th>
                <th className="text-base font-semibold uppercase tracking-wider text-base-content/50">Pricing Policy</th>
                <th className="text-base font-semibold uppercase tracking-wider text-base-content/50">Markup</th>
                <th className="text-base font-semibold uppercase tracking-wider text-base-content/50">Status</th>
                <th className="text-base font-semibold uppercase tracking-wider text-base-content/50 w-20">Actions</th>
              </tr></thead>
              <tbody>
                {channelModels?.map((cm) => (
                  <tr key={cm.id} className="border-b border-base-200 hover">
                    <td className="mono text-base">{cm.model_id}</td>
                    <td className="mono text-base">{cm.upstream_model_name ?? '—'}</td>
                    <td className="mono">{cm.priority_override ?? '—'}</td>
                    <td className="mono text-base">{cm.pricing_policy_id ?? '—'}</td>
                    <td className="mono">{cm.markup_ratio.toFixed(2)}x</td>
                    <td><Badge variant={cm.enabled ? 'green' : 'red'}>{cm.enabled ? 'Active' : 'Disabled'}</Badge></td>
                    <td>
                      <div className="flex items-center gap-1">
                        <button onClick={() => openEditModel(cm)} className="btn btn-ghost btn-xs btn-circle" aria-label="Edit model">
                          <Pencil className="h-3.5 w-3.5" />
                        </button>
                        <ConfirmDialog title={`Remove model "${cm.model_id}" from this provider?`} onConfirm={() => deleteModelMutation.mutateAsync(cm.id)} okText="Delete">
                          <button className="btn btn-ghost btn-xs btn-circle text-error hover:text-error" aria-label="Delete model">
                            <Trash2 className="h-3.5 w-3.5" />
                          </button>
                        </ConfirmDialog>
                      </div>
                    </td>
                  </tr>
                ))}
                {(!channelModels?.length) && (
                  <tr>
                    <td colSpan={7} className="text-center py-12">
                      <div className="flex flex-col items-center gap-2">
                        <span className="text-base-content/25 text-md">No models configured</span>
                        <button onClick={openAddModel} className="link link-primary text-md">Add your first model</button>
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
          <div className="flex items-center justify-between mb-3"><h2 className="text-lg font-semibold">Channels</h2><Button icon={<Plus className="h-4 w-4" />} onClick={openAddChannel}>Add Channel</Button></div>
          <div className="overflow-x-auto bg-base-100 rounded-box shadow-sm">
            <table className="table table-sm">
              <thead><tr className="border-b border-base-300"><th className="text-base font-semibold uppercase tracking-wider text-base-content/50">Name</th><th className="text-base font-semibold uppercase tracking-wider text-base-content/50">Priority</th><th className="text-base font-semibold uppercase tracking-wider text-base-content/50">Status</th><th className="text-base font-semibold uppercase tracking-wider text-base-content/50 w-20">Actions</th></tr></thead>
              <tbody>
                {channels?.map((channel) => (
                  <tr key={channel.id} className="border-b border-base-200 hover">
                    <td>{channel.name}</td>
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
                    <td colSpan={4} className="text-center py-12">
                      <div className="flex flex-col items-center gap-2">
                        <span className="text-base-content/25 text-md">No channels configured</span>
                        <button onClick={openAddChannel} className="link link-primary text-md">Add your first channel</button>
                      </div>
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </div>
      )}

      <Modal open={modelModalOpen} onClose={() => setModelModalOpen(false)} title={editingChannelModel ? 'Edit Channel Model' : 'Add Channel Model'}>
        <form onSubmit={handleSaveModel} className="space-y-4">
          {!editingChannelModel && (
            <div className="form-control">
              <label className="label"><span className="label-text">Model ID</span></label>
              <input type="text" value={modelId} onChange={(e) => setModelId(e.target.value)} placeholder="e.g., gpt-4o" required className="input input-bordered w-full" />
            </div>
          )}
          <div className="form-control">
            <label className="label"><span className="label-text">Upstream Model Name</span></label>
            <input type="text" value={upstreamModelName} onChange={(e) => setUpstreamModelName(e.target.value)} placeholder="Leave blank to use model ID" className="input input-bordered w-full" />
          </div>
          <div className="form-control">
            <label className="label"><span className="label-text">Priority Override</span></label>
            <input type="number" value={priorityOverride} onChange={(e) => setPriorityOverride(e.target.value)} placeholder="Use channel default" className="input input-bordered w-full" />
          </div>
          <div className="form-control">
            <label className="label"><span className="label-text">Pricing Policy</span></label>
            <Select
              value={pricingPolicyId}
              onChange={(v) => setPricingPolicyId(v as string)}
              options={[{ value: '', label: 'No policy' }, ...(policies ?? []).map(p => ({ value: p.id, label: p.name }))]}
            />
          </div>
          <div className="form-control">
            <label className="label"><span className="label-text">Markup Ratio</span></label>
            <input type="number" value={markupRatio} onChange={(e) => setMarkupRatio(e.target.value)} min={0} step={0.1} className="input input-bordered w-full" />
          </div>
          {editingChannelModel && (
            <div className="flex items-center justify-between">
              <label className="label-text">Enabled</label>
              <Toggle checked={modelEnabled} onChange={setModelEnabled} />
            </div>
          )}
          <Button variant="primary" type="submit" loading={createModelMutation.isPending || updateModelMutation.isPending}>{editingChannelModel ? 'Update' : 'Create'}</Button>
        </form>
      </Modal>

      <Modal open={channelModalOpen} onClose={() => setChannelModalOpen(false)} title={editingChannel ? `Edit Channel: ${editingChannel.name}` : 'Add Channel'}>
        <form onSubmit={handleSaveChannel} className="space-y-4">
          <div className="form-control"><label className="label"><span className="label-text">Name</span></label><input type="text" value={channelName} onChange={(e) => setChannelName(e.target.value)} placeholder="e.g., primary" required className="input input-bordered w-full" /></div>
          <div className="form-control"><label className="label"><span className="label-text">API Key</span></label><input type="password" value={channelApiKey} onChange={(e) => setChannelApiKey(e.target.value)} placeholder="Upstream API key" required className="input input-bordered w-full" /></div>
          <div className="form-control"><label className="label"><span className="label-text">Priority</span></label><input type="number" value={channelPriority} onChange={(e) => setChannelPriority(e.target.value)} min={0} className="input input-bordered w-full" /></div>
          {editingChannel && (<div className="flex items-center justify-between"><label className="label-text">Enabled</label><Toggle checked={channelEnabled} onChange={setChannelEnabled} /></div>)}
          <Button variant="primary" type="submit" loading={createChannelMutation.isPending || updateChannelMutation.isPending}>{editingChannel ? 'Update' : 'Create'}</Button>
        </form>
      </Modal>
    </div>
  );
}
