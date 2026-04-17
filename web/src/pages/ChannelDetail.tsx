import { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { ArrowLeft, Pencil, Trash2, KeyRound, Globe, Hash, Plus, Building2, LinkIcon, Power } from 'lucide-react';
import { useChannel, useUpdateChannel, useDeleteChannel, useChannelModels, useCreateChannelModel, useDeleteChannelModel } from '../hooks/useChannels';
import { useProviders } from '../hooks/useProviders';
import { useAllModels } from '../hooks/useModels';
import { Button } from '../components/ui/Button';
import { Modal } from '../components/ui/Modal';
import { Toggle } from '../components/ui/Toggle';
import { ConfirmDialog } from '../components/ui/ConfirmDialog';
import type { UpdateChannelRequest } from '../types';

export default function ChannelDetail() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { data: channel, isLoading } = useChannel(id!);
  const { data: providers } = useProviders();
  const { data: channelModels, isLoading: modelsLoading } = useChannelModels(channel?.id || '');
  const { data: allModels } = useAllModels();
  const updateMutation = useUpdateChannel(id!);
  const deleteMutation = useDeleteChannel(id!);
  const createModelMutation = useCreateChannelModel(channel?.id || '');
  const deleteModelMutation = useDeleteChannelModel(channel?.id || '');

  const [isEditing, setIsEditing] = useState(false);
  const [channelName, setChannelName] = useState('');
  const [channelApiKey, setChannelApiKey] = useState('');
  const [channelBaseUrl, setChannelBaseUrl] = useState('');
  const [channelPriority, setChannelPriority] = useState('0');
  const [channelEnabled, setChannelEnabled] = useState(false);

  const [isAddingModel, setIsAddingModel] = useState(false);
  const [selectedModel, setSelectedModel] = useState('');
  const [upstreamModelName, setUpstreamModelName] = useState('');
  const [modelEnabled, setModelEnabled] = useState(true);

  useEffect(() => {
    if (channel) {
      setChannelName(channel.name);
      setChannelApiKey(channel.api_key);
      setChannelBaseUrl(channel.base_url ?? '');
      setChannelPriority(String(channel.priority));
      setChannelEnabled(channel.enabled);
    }
  }, [channel]);

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-12">
        <span className="loading loading-spinner loading-lg" />
      </div>
    );
  }

  if (!channel) {
    return (
      <div className="text-center py-12">
        <div className="text-base-content/40 mb-4">Channel not found</div>
        <Button variant="secondary" onClick={() => navigate('/console/channels')}>
          Back to Channels
        </Button>
      </div>
    );
  }

  const provider = providers?.find(p => p.id === channel.provider_id);

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    const input: UpdateChannelRequest = {
      name: channelName,
      api_key: channelApiKey,
      base_url: channelBaseUrl || null,
      priority: Number(channelPriority),
      enabled: channelEnabled,
    };
    await updateMutation.mutateAsync({ id: channel.id, input });
    setIsEditing(false);
  };

  const handleDelete = async () => {
    await deleteMutation.mutateAsync(channel.id);
    navigate('/console/channels');
  };

  const handleCancelEdit = () => {
    setChannelName(channel.name);
    setChannelApiKey(channel.api_key);
    setChannelBaseUrl(channel.base_url ?? '');
    setChannelPriority(String(channel.priority));
    setChannelEnabled(channel.enabled);
    setIsEditing(false);
  };

  return (
    <div>
      <Button variant="ghost" icon={<ArrowLeft className="h-4 w-4" />} onClick={() => navigate('/console/channels')} className="mb-4">
        Back to Channels
      </Button>

      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold">Channel: {channel.name}</h1>
          <p className="text-sm text-base-content/40 mt-1">
            Provider: {provider?.name ?? channel.provider_id}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="secondary" icon={<Pencil className="h-4 w-4" />} onClick={() => setIsEditing(true)}>
            Edit
          </Button>
          <ConfirmDialog title={`Delete channel "${channel.name}"?`} onConfirm={handleDelete} okText="Delete">
            <Button variant="danger" icon={<Trash2 className="h-4 w-4" />}>Delete</Button>
          </ConfirmDialog>
        </div>
      </div>

      <div className="grid gap-6 max-w-2xl">
        {/* Status Card */}
        <div className="bg-base-100 rounded-box p-5 shadow-sm">
          <h2 className="text-sm font-semibold text-base-content/60 mb-4">Status</h2>
          <div className="flex items-center gap-3">
            <span className={`inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full text-sm font-medium ${
              channel.enabled
                ? 'bg-success/10 text-success'
                : 'bg-base-300/50 text-base-content/40'
            }`}>
              <span className={`w-2 h-2 rounded-full ${channel.enabled ? 'bg-success' : 'bg-base-content/30'}`} />
              {channel.enabled ? 'Active' : 'Disabled'}
            </span>
          </div>
        </div>

        {/* Details Card */}
        <div className="bg-base-100 rounded-box p-5 shadow-sm">
          <h2 className="text-sm font-semibold text-base-content/60 mb-4">Configuration</h2>
          <div className="space-y-4">
            <div className="flex items-center gap-3">
              <div className="w-8 h-8 rounded-lg bg-primary/10 flex items-center justify-center shrink-0">
                <Globe className="h-4 w-4 text-primary" />
              </div>
              <div className="flex-1 min-w-0">
                <div className="text-xs text-base-content/40 uppercase tracking-wider">Base URL</div>
                <div className="text-sm font-mono text-base-content/80 truncate" title={channel.base_url ?? 'Provider default'}>
                  {channel.base_url || <span className="text-base-content/40">Provider default</span>}
                </div>
              </div>
            </div>

            <div className="flex items-center gap-3">
              <div className="w-8 h-8 rounded-lg bg-primary/10 flex items-center justify-center shrink-0">
                <KeyRound className="h-4 w-4 text-primary" />
              </div>
              <div className="flex-1 min-w-0">
                <div className="text-xs text-base-content/40 uppercase tracking-wider">API Key</div>
                <div className="text-sm font-mono text-base-content/80">
                  {'•'.repeat(24)}
                </div>
              </div>
            </div>

            <div className="flex items-center gap-3">
              <div className="w-8 h-8 rounded-lg bg-primary/10 flex items-center justify-center shrink-0">
                <Hash className="h-4 w-4 text-primary" />
              </div>
              <div className="flex-1">
                <div className="text-xs text-base-content/40 uppercase tracking-wider">Priority</div>
                <div className="text-sm font-mono text-base-content/80">
                  {channel.priority}
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* Provider Card */}
        {provider && (
          <div className="bg-base-100 rounded-box p-5 shadow-sm">
            <h2 className="text-sm font-semibold text-base-content/60 mb-4">Provider</h2>
            <div className="space-y-4">
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 rounded-lg bg-secondary/10 flex items-center justify-center shrink-0">
                  <Building2 className="h-4 w-4 text-secondary" />
                </div>
                <div className="flex-1 min-w-0">
                  <div className="text-xs text-base-content/40 uppercase tracking-wider">Name</div>
                  <div className="text-sm font-medium">{provider.name}</div>
                </div>
              </div>

              {provider.base_url && (
                <div className="flex items-center gap-3">
                  <div className="w-8 h-8 rounded-lg bg-secondary/10 flex items-center justify-center shrink-0">
                    <Globe className="h-4 w-4 text-secondary" />
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="text-xs text-base-content/40 uppercase tracking-wider">Base URL</div>
                    <div className="text-sm font-mono text-base-content/80 truncate" title={provider.base_url}>
                      {provider.base_url}
                    </div>
                  </div>
                </div>
              )}

              {provider.endpoints && (
                <div className="flex items-center gap-3">
                  <div className="w-8 h-8 rounded-lg bg-secondary/10 flex items-center justify-center shrink-0">
                    <LinkIcon className="h-4 w-4 text-secondary" />
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="text-xs text-base-content/40 uppercase tracking-wider">Endpoints</div>
                    <div className="text-sm font-mono text-base-content/80 truncate" title={provider.endpoints}>
                      {provider.endpoints}
                    </div>
                  </div>
                </div>
              )}

              <div className="flex items-center gap-3">
                <div className="w-8 h-8 rounded-lg bg-secondary/10 flex items-center justify-center shrink-0">
                  <Power className="h-4 w-4 text-secondary" />
                </div>
                <div className="flex-1">
                  <div className="text-xs text-base-content/40 uppercase tracking-wider">Status</div>
                  <span className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded text-xs font-medium ${
                    provider.enabled
                      ? 'bg-success/10 text-success'
                      : 'bg-base-300/50 text-base-content/40'
                  }`}>
                    {provider.enabled ? 'Enabled' : 'Disabled'}
                  </span>
                </div>
              </div>
            </div>
          </div>
        )}
        {/* Metadata Card */}
        <div className="bg-base-100 rounded-box p-5 shadow-sm">
          <h2 className="text-sm font-semibold text-base-content/60 mb-4">Metadata</h2>
          <div className="grid grid-cols-2 gap-4 text-sm">
            <div>
              <div className="text-xs text-base-content/40 uppercase tracking-wider mb-1">ID</div>
              <div className="font-mono text-base-content/60 text-xs truncate" title={channel.id}>
                {channel.id}
              </div>
            </div>
            <div>
              <div className="text-xs text-base-content/40 uppercase tracking-wider mb-1">Created</div>
              <div className="text-base-content/60">
                {new Date(channel.created_at).toLocaleDateString()}
              </div>
            </div>
            <div>
              <div className="text-xs text-base-content/40 uppercase tracking-wider mb-1">Updated</div>
              <div className="text-base-content/60">
                {new Date(channel.updated_at).toLocaleDateString()}
              </div>
            </div>
          </div>
        </div>

        {/* Channel Models Card */}
        <div className="bg-base-100 rounded-box p-5 shadow-sm">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-sm font-semibold text-base-content/60">Channel Models</h2>
            <Button variant="secondary" size="sm" icon={<Plus className="h-4 w-4" />} onClick={() => setIsAddingModel(true)}>
              Add Model
            </Button>
          </div>

          {modelsLoading ? (
            <div className="flex items-center justify-center py-8">
              <span className="loading loading-spinner loading-sm" />
            </div>
          ) : channelModels && channelModels.length > 0 ? (
            <div className="space-y-2">
              {channelModels.map((cm) => (
                <div key={cm.id} className="flex items-center justify-between p-3 rounded-lg bg-base-200/50">
                  <div className="flex-1 min-w-0">
                    <div className="font-mono text-sm truncate">{cm.upstream_model_name}</div>
                    <div className="text-xs text-base-content/40 mt-0.5">
                      Model ID: {cm.model_id}
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className={`inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium ${
                      cm.enabled
                        ? 'bg-success/10 text-success'
                        : 'bg-base-300/50 text-base-content/40'
                    }`}>
                      {cm.enabled ? 'Active' : 'Disabled'}
                    </span>
                    <ConfirmDialog title={`Remove model "${cm.upstream_model_name}" from this channel?`} onConfirm={() => deleteModelMutation.mutateAsync(cm.id)} okText="Remove">
                      <Button variant="ghost" size="sm" icon={<Trash2 className="h-4 w-4" />} />
                    </ConfirmDialog>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className="text-center py-8 text-base-content/40">
              <p className="text-sm">No models added to this channel</p>
              <Button variant="ghost" size="sm" className="mt-2" onClick={() => setIsAddingModel(true)}>
                Add your first model
              </Button>
            </div>
          )}
        </div>
      </div>

      {/* Edit Modal */}
      <Modal open={isEditing} onClose={handleCancelEdit} title="Edit Channel">
        <form onSubmit={handleSave} className="space-y-4">
          <div className="form-control">
            <label className="label">
              <span className="label-text">Name</span>
            </label>
            <input
              type="text"
              value={channelName}
              onChange={(e) => setChannelName(e.target.value)}
              required
              className="input input-bordered w-full"
            />
          </div>

          <div className="form-control">
            <label className="label">
              <span className="label-text">API Key</span>
            </label>
            <input
              type="password"
              value={channelApiKey}
              onChange={(e) => setChannelApiKey(e.target.value)}
              placeholder="Leave empty to keep current"
              className="input input-bordered w-full"
            />
            <label className="label">
              <span className="label-text-alt">Leave empty to keep current API key</span>
            </label>
          </div>

          <div className="form-control">
            <label className="label">
              <span className="label-text">Base URL</span>
            </label>
            <input
              type="text"
              value={channelBaseUrl}
              onChange={(e) => setChannelBaseUrl(e.target.value)}
              placeholder="Leave empty to use provider default"
              className="input input-bordered w-full"
            />
          </div>

          <div className="form-control">
            <label className="label">
              <span className="label-text">Priority</span>
            </label>
            <input
              type="number"
              value={channelPriority}
              onChange={(e) => setChannelPriority(e.target.value)}
              min={0}
              className="input input-bordered w-full"
            />
          </div>

          <div className="flex items-center justify-between">
            <span className="label-text">Enabled</span>
            <Toggle checked={channelEnabled} onChange={setChannelEnabled} />
          </div>

          <div className="flex gap-2 pt-2">
            <Button variant="primary" type="submit" loading={updateMutation.isPending}>
              Save Changes
            </Button>
            <Button variant="ghost" type="button" onClick={handleCancelEdit}>
              Cancel
            </Button>
          </div>
        </form>
      </Modal>

      {/* Add Model Modal */}
      <Modal open={isAddingModel} onClose={() => setIsAddingModel(false)} title="Add Model to Channel">
        <form onSubmit={async (e) => {
          e.preventDefault();
          if (!selectedModel || !upstreamModelName) return;
          await createModelMutation.mutateAsync({
            model_id: selectedModel,
            upstream_model_name: upstreamModelName,
            enabled: modelEnabled,
          });
          setIsAddingModel(false);
          setSelectedModel('');
          setUpstreamModelName('');
          setModelEnabled(true);
        }} className="space-y-4">
          <div className="form-control">
            <label className="label">
              <span className="label-text">Select Model</span>
            </label>
            <select
              value={selectedModel}
              onChange={(e) => setSelectedModel(e.target.value)}
              required
              className="select select-bordered w-full"
            >
              <option value="">Select a model...</option>
              {allModels?.filter(m => !channelModels?.some(cm => cm.model_id === m.id)).map((model) => (
                <option key={model.id} value={model.id}>
                  {model.name}
                </option>
              ))}
            </select>
          </div>

          <div className="form-control">
            <label className="label">
              <span className="label-text">Upstream Model Name</span>
            </label>
            <input
              type="text"
              value={upstreamModelName}
              onChange={(e) => setUpstreamModelName(e.target.value)}
              placeholder="e.g., gpt-4o, claude-3-opus"
              className="input input-bordered w-full"
            />
            <label className="label">
              <span className="label-text-alt">The exact model name the provider expects</span>
            </label>
          </div>

          <div className="flex items-center justify-between">
            <span className="label-text">Enabled</span>
            <Toggle checked={modelEnabled} onChange={setModelEnabled} />
          </div>

          <div className="flex gap-2 pt-2">
            <Button variant="primary" type="submit" loading={createModelMutation.isPending}>
              Add Model
            </Button>
            <Button variant="ghost" type="button" onClick={() => setIsAddingModel(false)}>
              Cancel
            </Button>
          </div>
        </form>
      </Modal>
    </div>
  );
}