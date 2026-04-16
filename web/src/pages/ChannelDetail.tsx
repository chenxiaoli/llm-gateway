import { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { ArrowLeft, Pencil, Trash2, KeyRound, Globe, Hash } from 'lucide-react';
import { useChannel, useUpdateChannel, useDeleteChannel } from '../hooks/useProviders';
import { useProviders } from '../hooks/useProviders';
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
  const updateMutation = useUpdateChannel(id!);
  const deleteMutation = useDeleteChannel(id!);

  const [isEditing, setIsEditing] = useState(false);
  const [channelName, setChannelName] = useState('');
  const [channelApiKey, setChannelApiKey] = useState('');
  const [channelBaseUrl, setChannelBaseUrl] = useState('');
  const [channelPriority, setChannelPriority] = useState('0');
  const [channelEnabled, setChannelEnabled] = useState(false);

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
    </div>
  );
}