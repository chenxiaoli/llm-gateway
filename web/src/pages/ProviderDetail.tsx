import { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { ArrowLeft, Plus } from 'lucide-react';
import { useProvider, useUpdateProvider, useDeleteProvider, useChannels, useCreateChannel, useUpdateChannel, useDeleteChannel } from '../hooks/useProviders';
import { useModels, useCreateModel, useUpdateModel, useDeleteModel } from '../hooks/useModels';
import { Button } from '../components/ui/Button';
import { Modal } from '../components/ui/Modal';
import { Badge } from '../components/ui/Badge';
import { Toggle } from '../components/ui/Toggle';
import { Select } from '../components/ui/Select';
import { ConfirmDialog } from '../components/ui/ConfirmDialog';
import type { Model, CreateModelRequest, UpdateModelRequest, Channel } from '../types';

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

  const { data: channels } = useChannels(id!);
  const { data: models } = useModels(id!);

  // Provider form state
  const [provName, setProvName] = useState('');
  const [provOpenaiUrl, setProvOpenaiUrl] = useState('');
  const [provAnthropicUrl, setProvAnthropicUrl] = useState('');
  const [provEnabled, setProvEnabled] = useState(false);

  // Model modal state
  const [modelModalOpen, setModelModalOpen] = useState(false);
  const [editingModel, setEditingModel] = useState<Model | null>(null);
  const [modelName, setModelName] = useState('');
  const [modelBillingType, setModelBillingType] = useState<'token' | 'request'>('token');
  const [modelInputPrice, setModelInputPrice] = useState('');
  const [modelOutputPrice, setModelOutputPrice] = useState('');
  const [modelRequestPrice, setModelRequestPrice] = useState('');
  const [modelEnabled, setModelEnabled] = useState(false);

  // Channel modal state
  const [channelModalOpen, setChannelModalOpen] = useState(false);
  const [editingChannel, setEditingChannel] = useState<Channel | null>(null);
  const [channelName, setChannelName] = useState('');
  const [channelApiKey, setChannelApiKey] = useState('');
  const [channelBaseUrl, setChannelBaseUrl] = useState('');
  const [channelPriority, setChannelPriority] = useState('0');
  const [channelEnabled, setChannelEnabled] = useState(false);

  useEffect(() => {
    if (provider) {
      setProvName(provider.name);
      setProvOpenaiUrl(provider.openai_base_url ?? '');
      setProvAnthropicUrl(provider.anthropic_base_url ?? '');
      setProvEnabled(provider.enabled);
    }
  }, [provider]);

  if (isLoading) return <div className="text-[#555555]">Loading...</div>;
  if (!provider) return <div className="text-[#555555]">Provider not found</div>;

  const handleUpdateProvider = async (e: React.FormEvent) => {
    e.preventDefault();
    await updateMutation.mutateAsync({
      id: provider.id,
      input: {
        name: provName,
        openai_base_url: provOpenaiUrl || null,
        anthropic_base_url: provAnthropicUrl || null,
        enabled: provEnabled,
      },
    });
  };

  const handleDeleteProvider = async () => {
    await deleteMutation.mutateAsync(provider.id);
    navigate('/console/providers');
  };

  const resetModelForm = () => {
    setModelName('');
    setModelBillingType('token');
    setModelInputPrice('');
    setModelOutputPrice('');
    setModelRequestPrice('');
    setModelEnabled(false);
  };

  const openAddModel = () => {
    setEditingModel(null);
    resetModelForm();
    setModelModalOpen(true);
  };

  const openEditModel = (model: Model) => {
    setEditingModel(model);
    setModelName(model.name);
    setModelBillingType(model.billing_type);
    setModelInputPrice(String(model.input_price));
    setModelOutputPrice(String(model.output_price));
    setModelRequestPrice(String(model.request_price));
    setModelEnabled(model.enabled);
    setModelModalOpen(true);
  };

  const handleSaveModel = async (e: React.FormEvent) => {
    e.preventDefault();
    if (editingModel) {
      const updateInput: UpdateModelRequest = {
        billing_type: modelBillingType,
        input_price: Number(modelInputPrice) || 0,
        output_price: Number(modelOutputPrice) || 0,
        request_price: Number(modelRequestPrice) || 0,
        enabled: modelEnabled,
      };
      await updateModelMutation.mutateAsync({ modelName: editingModel.name, input: updateInput });
    } else {
      const input: CreateModelRequest = {
        name: modelName,
        billing_type: modelBillingType,
        input_price: Number(modelInputPrice) || 0,
        output_price: Number(modelOutputPrice) || 0,
        request_price: Number(modelRequestPrice) || 0,
      };
      await createModelMutation.mutateAsync(input);
    }
    setModelModalOpen(false);
  };

  const resetChannelForm = () => {
    setChannelName('');
    setChannelApiKey('');
    setChannelBaseUrl('');
    setChannelPriority('0');
    setChannelEnabled(false);
  };

  const openAddChannel = () => {
    setEditingChannel(null);
    resetChannelForm();
    setChannelModalOpen(true);
  };

  const openEditChannel = (channel: Channel) => {
    setEditingChannel(channel);
    setChannelName(channel.name);
    setChannelApiKey(channel.api_key);
    setChannelBaseUrl(channel.base_url ?? '');
    setChannelPriority(String(channel.priority));
    setChannelEnabled(channel.enabled);
    setChannelModalOpen(true);
  };

  const handleSaveChannel = async (e: React.FormEvent) => {
    e.preventDefault();
    if (editingChannel) {
      await updateChannelMutation.mutateAsync({
        id: editingChannel.id,
        input: {
          name: channelName,
          api_key: channelApiKey,
          base_url: channelBaseUrl || null,
          priority: Number(channelPriority),
          enabled: channelEnabled,
        },
      });
    } else {
      await createChannelMutation.mutateAsync({
        name: channelName,
        api_key: channelApiKey,
        base_url: channelBaseUrl || null,
        priority: Number(channelPriority),
      });
    }
    setChannelModalOpen(false);
  };

  return (
    <div>
      <Button variant="ghost" icon={<ArrowLeft className="h-4 w-4" />} onClick={() => navigate('/console/providers')} className="mb-4">
        Back to Providers
      </Button>

      <div className="mb-6">
        <h1 className="font-display text-2xl font-bold text-[#ededed]">Provider: {provider.name}</h1>
      </div>

      {/* Provider form */}
      <form onSubmit={handleUpdateProvider} className="mb-8 max-w-lg rounded-xl border border-[#1e1e1e] bg-[#0a0a0a] p-5 space-y-4">
        <div>
          <label className="block text-xs font-medium text-[#888888] mb-1.5">Name</label>
          <input type="text" value={provName} onChange={(e) => setProvName(e.target.value)} required
            className="h-9 w-full rounded-lg border border-[#262626] bg-[#141414] px-3 text-sm text-[#ededed] outline-none focus:border-accent/50 transition-colors" />
        </div>
        <div>
          <label className="block text-xs font-medium text-[#888888] mb-1.5">OpenAI Base URL</label>
          <input type="text" value={provOpenaiUrl} onChange={(e) => setProvOpenaiUrl(e.target.value)}
            className="h-9 w-full rounded-lg border border-[#262626] bg-[#141414] px-3 text-sm text-[#ededed] placeholder-[#555555] outline-none focus:border-accent/50 transition-colors" />
        </div>
        <div>
          <label className="block text-xs font-medium text-[#888888] mb-1.5">Anthropic Base URL</label>
          <input type="text" value={provAnthropicUrl} onChange={(e) => setProvAnthropicUrl(e.target.value)}
            className="h-9 w-full rounded-lg border border-[#262626] bg-[#141414] px-3 text-sm text-[#ededed] placeholder-[#555555] outline-none focus:border-accent/50 transition-colors" />
        </div>
        <div className="flex items-center justify-between">
          <label className="text-xs font-medium text-[#888888]">Enabled</label>
          <Toggle checked={provEnabled} onChange={setProvEnabled} />
        </div>
        <div className="flex gap-2">
          <Button variant="primary" type="submit" loading={updateMutation.isPending}>Save</Button>
          <ConfirmDialog title="Delete this provider and all its models?" onConfirm={handleDeleteProvider} okText="Delete">
            <Button variant="danger">Delete Provider</Button>
          </ConfirmDialog>
        </div>
      </form>

      {/* Models */}
      <div className="mb-8">
        <div className="flex items-center justify-between mb-3">
          <h2 className="font-display text-base font-semibold text-[#ededed]">Models</h2>
          <Button icon={<Plus className="h-4 w-4" />} onClick={openAddModel}>Add Model</Button>
        </div>
        <div className="overflow-x-auto rounded-xl border border-[#1e1e1e]">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-[#1e1e1e]">
                <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Name</th>
                <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Billing</th>
                <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Input ($/1M)</th>
                <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Output ($/1M)</th>
                <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Status</th>
                <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Actions</th>
              </tr>
            </thead>
            <tbody>
              {models?.map((model) => (
                <tr key={model.name} className="border-b border-[#1e1e1e]/50 hover:bg-white/[0.02] transition-colors">
                  <td className="px-4 py-2.5"><span className="mono">{model.name}</span></td>
                  <td className="px-4 py-2.5"><Badge variant={model.billing_type === 'token' ? 'blue' : 'green'}>{model.billing_type}</Badge></td>
                  <td className="px-4 py-2.5"><span className="mono">{model.input_price.toFixed(2)}</span></td>
                  <td className="px-4 py-2.5"><span className="mono">{model.output_price.toFixed(2)}</span></td>
                  <td className="px-4 py-2.5"><Badge variant={model.enabled ? 'green' : 'red'}>{model.enabled ? 'Active' : 'Disabled'}</Badge></td>
                  <td className="px-4 py-2.5">
                    <div className="flex gap-2">
                      <button onClick={() => openEditModel(model)} className="text-accent hover:text-accent-hover transition-colors text-sm">Edit</button>
                      <ConfirmDialog title={`Delete model "${model.name}"?`} onConfirm={() => deleteModelMutation.mutateAsync(model.name)} okText="Delete">
                        <button className="text-danger hover:text-danger/80 transition-colors text-sm">Delete</button>
                      </ConfirmDialog>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* Channels */}
      <div className="mb-8">
        <div className="flex items-center justify-between mb-3">
          <h2 className="font-display text-base font-semibold text-[#ededed]">Channels</h2>
          <Button icon={<Plus className="h-4 w-4" />} onClick={openAddChannel}>Add Channel</Button>
        </div>
        <div className="overflow-x-auto rounded-xl border border-[#1e1e1e]">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-[#1e1e1e]">
                <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Name</th>
                <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Base URL</th>
                <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Priority</th>
                <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Status</th>
                <th className="px-4 py-3 text-left text-[11px] font-semibold uppercase tracking-[0.05em] text-[#555555]">Actions</th>
              </tr>
            </thead>
            <tbody>
              {channels?.map((channel) => (
                <tr key={channel.id} className="border-b border-[#1e1e1e]/50 hover:bg-white/[0.02] transition-colors">
                  <td className="px-4 py-2.5">{channel.name}</td>
                  <td className="px-4 py-2.5">
                    {channel.base_url ? <span className="mono">{channel.base_url}</span> : <span className="text-[#555555]">Default</span>}
                  </td>
                  <td className="px-4 py-2.5"><span className="mono">{channel.priority}</span></td>
                  <td className="px-4 py-2.5"><Badge variant={channel.enabled ? 'green' : 'red'}>{channel.enabled ? 'Active' : 'Disabled'}</Badge></td>
                  <td className="px-4 py-2.5">
                    <div className="flex gap-2">
                      <button onClick={() => openEditChannel(channel)} className="text-accent hover:text-accent-hover transition-colors text-sm">Edit</button>
                      <ConfirmDialog title={`Delete channel "${channel.name}"?`} onConfirm={() => deleteChannelMutation.mutateAsync(channel.id)} okText="Delete">
                        <button className="text-danger hover:text-danger/80 transition-colors text-sm">Delete</button>
                      </ConfirmDialog>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* Model Modal */}
      <Modal open={modelModalOpen} onClose={() => setModelModalOpen(false)} title={editingModel ? `Edit Model: ${editingModel.name}` : 'Add Model'}>
        <form onSubmit={handleSaveModel} className="space-y-4">
          {!editingModel && (
            <div>
              <label className="block text-xs font-medium text-[#888888] mb-1.5">Model Name</label>
              <input type="text" value={modelName} onChange={(e) => setModelName(e.target.value)} placeholder="e.g., gpt-4o" required
                className="h-9 w-full rounded-lg border border-[#262626] bg-[#141414] px-3 text-sm text-[#ededed] placeholder-[#555555] outline-none focus:border-accent/50 transition-colors" />
            </div>
          )}
          <div>
            <label className="block text-xs font-medium text-[#888888] mb-1.5">Billing Type</label>
            <Select value={modelBillingType} onChange={(v) => setModelBillingType(v as 'token' | 'request')} options={[
              { value: 'token', label: 'Token-based' },
              { value: 'request', label: 'Request-based' },
            ]} />
          </div>
          <div className="flex gap-4">
            <div className="flex-1">
              <label className="block text-xs font-medium text-[#888888] mb-1.5">Input ($/1M)</label>
              <input type="number" value={modelInputPrice} onChange={(e) => setModelInputPrice(e.target.value)} min={0} step={0.01}
                className="h-9 w-full rounded-lg border border-[#262626] bg-[#141414] px-3 text-sm text-[#ededed] placeholder-[#555555] outline-none focus:border-accent/50 transition-colors" />
            </div>
            <div className="flex-1">
              <label className="block text-xs font-medium text-[#888888] mb-1.5">Output ($/1M)</label>
              <input type="number" value={modelOutputPrice} onChange={(e) => setModelOutputPrice(e.target.value)} min={0} step={0.01}
                className="h-9 w-full rounded-lg border border-[#262626] bg-[#141414] px-3 text-sm text-[#ededed] placeholder-[#555555] outline-none focus:border-accent/50 transition-colors" />
            </div>
            <div className="flex-1">
              <label className="block text-xs font-medium text-[#888888] mb-1.5">Request ($/req)</label>
              <input type="number" value={modelRequestPrice} onChange={(e) => setModelRequestPrice(e.target.value)} min={0} step={0.01}
                className="h-9 w-full rounded-lg border border-[#262626] bg-[#141414] px-3 text-sm text-[#ededed] placeholder-[#555555] outline-none focus:border-accent/50 transition-colors" />
            </div>
          </div>
          {editingModel && (
            <div className="flex items-center justify-between">
              <label className="text-xs font-medium text-[#888888]">Enabled</label>
              <Toggle checked={modelEnabled} onChange={setModelEnabled} />
            </div>
          )}
          <Button variant="primary" type="submit">
            {editingModel ? 'Update' : 'Create'}
          </Button>
        </form>
      </Modal>

      {/* Channel Modal */}
      <Modal open={channelModalOpen} onClose={() => setChannelModalOpen(false)} title={editingChannel ? `Edit Channel: ${editingChannel.name}` : 'Add Channel'}>
        <form onSubmit={handleSaveChannel} className="space-y-4">
          <div>
            <label className="block text-xs font-medium text-[#888888] mb-1.5">Name</label>
            <input type="text" value={channelName} onChange={(e) => setChannelName(e.target.value)} placeholder="e.g., primary" required
              className="h-9 w-full rounded-lg border border-[#262626] bg-[#141414] px-3 text-sm text-[#ededed] placeholder-[#555555] outline-none focus:border-accent/50 transition-colors" />
          </div>
          <div>
            <label className="block text-xs font-medium text-[#888888] mb-1.5">API Key</label>
            <input type="password" value={channelApiKey} onChange={(e) => setChannelApiKey(e.target.value)} placeholder="Upstream API key" required
              className="h-9 w-full rounded-lg border border-[#262626] bg-[#141414] px-3 text-sm text-[#ededed] placeholder-[#555555] outline-none focus:border-accent/50 transition-colors" />
          </div>
          <div>
            <label className="block text-xs font-medium text-[#888888] mb-1.5">Base URL</label>
            <input type="text" value={channelBaseUrl} onChange={(e) => setChannelBaseUrl(e.target.value)} placeholder="Leave empty to use provider default"
              className="h-9 w-full rounded-lg border border-[#262626] bg-[#141414] px-3 text-sm text-[#ededed] placeholder-[#555555] outline-none focus:border-accent/50 transition-colors" />
          </div>
          <div>
            <label className="block text-xs font-medium text-[#888888] mb-1.5">Priority</label>
            <input type="number" value={channelPriority} onChange={(e) => setChannelPriority(e.target.value)} min={0}
              className="h-9 w-full rounded-lg border border-[#262626] bg-[#141414] px-3 text-sm text-[#ededed] placeholder-[#555555] outline-none focus:border-accent/50 transition-colors" />
          </div>
          {editingChannel && (
            <div className="flex items-center justify-between">
              <label className="text-xs font-medium text-[#888888]">Enabled</label>
              <Toggle checked={channelEnabled} onChange={setChannelEnabled} />
            </div>
          )}
          <Button variant="primary" type="submit" loading={createChannelMutation.isPending || updateChannelMutation.isPending}>
            {editingChannel ? 'Update' : 'Create'}
          </Button>
        </form>
      </Modal>
    </div>
  );
}
