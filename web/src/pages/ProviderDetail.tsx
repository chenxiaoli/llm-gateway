import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { useParams, useNavigate } from 'react-router-dom';
import { ArrowLeft, Plus, Pencil, Trash2 } from 'lucide-react';
import { useProvider, useUpdateProvider, useDeleteProvider } from '../hooks/useProviders';
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
  const { t } = useTranslation();
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { data: provider, isLoading } = useProvider(id!);
  const updateMutation = useUpdateProvider();
  const deleteMutation = useDeleteProvider();
  const createChannelMutation = useCreateChannel(id!);
  const updateChannelMutation = useUpdateChannel(id!);
  const deleteChannelMutation = useDeleteChannel(id!);

  const { data: channels } = useChannels(id!);
  const { data: channelModels } = useChannelModels(id!);
  const createModelMutation = useCreateChannelModel(id!);
  const updateModelMutation = useUpdateChannelModel(id!);
  const deleteModelMutation = useDeleteChannelModel(id!);

  const { data: policies } = usePricingPolicies();

  const [provName, setProvName] = useState('');
  const [provDefaultUrl, setProvDefaultUrl] = useState('');
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
      let defaultUrl = '';
      let openaiUrl = '';
      let anthropicUrl = '';
      if (provider.endpoints) {
        defaultUrl = provider.endpoints.default || '';
        openaiUrl = provider.endpoints.openai || '';
        anthropicUrl = provider.endpoints.anthropic || '';
      }
      setProvDefaultUrl(defaultUrl);
      setProvOpenaiUrl(openaiUrl);
      setProvAnthropicUrl(anthropicUrl);
      setProvEnabled(provider.enabled);
      setProvProxyUrl(provider.proxy_url || '');
    }
  }, [provider]);

  if (isLoading) return <div className="flex justify-center py-12"><span className="loading loading-spinner loading-lg" /></div>;
  if (!provider) return <div className="text-base-content/40">{t('providerDetail.providerNotFound')}</div>;

  const handleUpdateProvider = async (e: React.FormEvent) => {
    e.preventDefault();
    const endpoints: Record<string, string | null> = {
      default: provDefaultUrl || null,
      openai: provOpenaiUrl || null,
      anthropic: provAnthropicUrl || null,
    };
    await updateMutation.mutateAsync({ id: provider.id, input: { name: provName, endpoints, proxy_url: provProxyUrl || null, enabled: provEnabled } });
  };

  const handleDeleteProvider = async () => {
    await deleteMutation.mutateAsync(provider.id);
    navigate('/admin/providers');
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
      <Button variant="ghost" icon={<ArrowLeft className="h-4 w-4" />} onClick={() => navigate('/admin/providers')} className="mb-4">{t('providerDetail.backToProviders')}</Button>
      <div className="mb-6"><h1 className="text-4xl font-bold">{t('providerDetail.pageTitle', { name: provider.name })}</h1></div>

      <form onSubmit={handleUpdateProvider} className="mb-8 max-w-lg bg-base-100 rounded-box p-5 shadow-sm space-y-4">
        <div className="form-control"><label className="label"><span className="label-text">{t('providerDetail.form.name')}</span></label><input type="text" value={provName} onChange={(e) => setProvName(e.target.value)} required className="input input-bordered w-full" /></div>
        <div className="form-control"><label className="label"><span className="label-text">{t('providerDetail.form.defaultEndpoint')}</span></label><input type="text" value={provDefaultUrl} onChange={(e) => setProvDefaultUrl(e.target.value)} placeholder="https://api.example.com/v1" className="input input-bordered w-full" /></div>
        <div className="form-control"><label className="label"><span className="label-text">{t('providerDetail.form.openaiEndpoint')}</span></label><input type="text" value={provOpenaiUrl} onChange={(e) => setProvOpenaiUrl(e.target.value)} placeholder="https://api.openai.com/v1" className="input input-bordered w-full" /></div>
        <div className="form-control"><label className="label"><span className="label-text">{t('providerDetail.form.anthropicEndpoint')}</span></label><input type="text" value={provAnthropicUrl} onChange={(e) => setProvAnthropicUrl(e.target.value)} placeholder="https://api.anthropic.com" className="input input-bordered w-full" /></div>
        <div className="form-control"><label className="label"><span className="label-text">{t('providerDetail.form.proxyUrl')}</span></label><input type="text" value={provProxyUrl} onChange={(e) => setProvProxyUrl(e.target.value)} placeholder="http://proxy:8080" className="input input-bordered w-full" /></div>
        <div className="flex items-center justify-between"><label className="label-text">{t('providerDetail.form.enabled')}</label><Toggle checked={provEnabled} onChange={setProvEnabled} /></div>
        <div className="flex gap-2">
          <Button variant="primary" type="submit" loading={updateMutation.isPending}>{t('common.save')}</Button>
          <ConfirmDialog title={t('providerDetail.confirmDeleteProvider')} onConfirm={handleDeleteProvider} okText={t('common.delete')}><Button variant="danger">{t('providerDetail.form.deleteProvider')}</Button></ConfirmDialog>
        </div>
      </form>

      {/* Tabs */}
      <div className="tabs tabs-boxed mb-6">
        <button className={`tab ${activeTab === 'models' ? 'tab-active' : ''}`} onClick={() => setActiveTab('models')}>{t('providerDetail.tabs.models')}</button>
        <button className={`tab ${activeTab === 'channels' ? 'tab-active' : ''}`} onClick={() => setActiveTab('channels')}>{t('providerDetail.tabs.channels')}</button>
      </div>

      {/* Models Tab */}
      {activeTab === 'models' && (
        <div className="mb-8">
          <div className="flex items-center justify-between mb-3">
            <h2 className="text-lg font-semibold">{t('providerDetail.models.title')}</h2>
            <div className="flex gap-2">
              <Button variant="ghost" icon={<Plus className="h-4 w-4" />} onClick={openAddModel}>{t('providerDetail.models.addModel')}</Button>
            </div>
          </div>
          <div className="overflow-x-auto bg-base-100 rounded-box shadow-sm">
            <table className="table table-sm">
              <thead><tr className="border-b border-base-300">
                <th className="text-base font-semibold uppercase tracking-wider text-base-content/50">{t('providerDetail.models.table.modelId')}</th>
                <th className="text-base font-semibold uppercase tracking-wider text-base-content/50">{t('providerDetail.models.table.upstreamName')}</th>
                <th className="text-base font-semibold uppercase tracking-wider text-base-content/50">{t('providerDetail.models.table.priority')}</th>
                <th className="text-base font-semibold uppercase tracking-wider text-base-content/50">{t('providerDetail.models.table.pricingPolicy')}</th>
                <th className="text-base font-semibold uppercase tracking-wider text-base-content/50">{t('providerDetail.models.table.markup')}</th>
                <th className="text-base font-semibold uppercase tracking-wider text-base-content/50">{t('providerDetail.models.table.status')}</th>
                <th className="text-base font-semibold uppercase tracking-wider text-base-content/50 w-20">{t('providerDetail.models.table.actions')}</th>
              </tr></thead>
              <tbody>
                {channelModels?.map((cm) => (
                  <tr key={cm.id} className="border-b border-base-200 hover">
                    <td className="mono text-base">{cm.model_id}</td>
                    <td className="mono text-base">{cm.upstream_model_name ?? '—'}</td>
                    <td className="mono">{cm.priority_override ?? '—'}</td>
                    <td className="mono text-base">{cm.pricing_policy_id ?? '—'}</td>
                    <td className="mono">{cm.markup_ratio.toFixed(2)}x</td>
                    <td><Badge variant={cm.enabled ? 'green' : 'red'}>{cm.enabled ? t('providerDetail.models.active') : t('providerDetail.models.disabled')}</Badge></td>
                    <td>
                      <div className="flex items-center gap-1">
                        <button onClick={() => openEditModel(cm)} className="btn btn-ghost btn-xs btn-circle" aria-label={t('common.edit')}>
                          <Pencil className="h-3.5 w-3.5" />
                        </button>
                        <ConfirmDialog title={t('providerDetail.models.confirmRemove', { modelId: cm.model_id })} onConfirm={() => deleteModelMutation.mutateAsync(cm.id)} okText={t('common.delete')}>
                          <button className="btn btn-ghost btn-xs btn-circle text-error hover:text-error" aria-label={t('common.delete')}>
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
                        <span className="text-base-content/25 text-md">{t('providerDetail.models.empty')}</span>
                        <button onClick={openAddModel} className="link link-primary text-md">{t('providerDetail.models.addFirst')}</button>
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
          <div className="flex items-center justify-between mb-3"><h2 className="text-lg font-semibold">{t('providerDetail.channels.title')}</h2><Button icon={<Plus className="h-4 w-4" />} onClick={openAddChannel}>{t('providerDetail.channels.addChannel')}</Button></div>
          <div className="overflow-x-auto bg-base-100 rounded-box shadow-sm">
            <table className="table table-sm">
              <thead><tr className="border-b border-base-300"><th className="text-base font-semibold uppercase tracking-wider text-base-content/50">{t('providerDetail.channels.table.name')}</th><th className="text-base font-semibold uppercase tracking-wider text-base-content/50">{t('providerDetail.channels.table.priority')}</th><th className="text-base font-semibold uppercase tracking-wider text-base-content/50">{t('providerDetail.channels.table.status')}</th><th className="text-base font-semibold uppercase tracking-wider text-base-content/50 w-20">{t('providerDetail.channels.table.actions')}</th></tr></thead>
              <tbody>
                {channels?.map((channel) => (
                  <tr key={channel.id} className="border-b border-base-200 hover">
                    <td>{channel.name}</td>
                    <td className="mono">{channel.priority}</td>
                    <td><Badge variant={channel.enabled ? 'green' : 'red'}>{channel.enabled ? t('providerDetail.channels.active') : t('providerDetail.channels.disabled')}</Badge></td>
                    <td>
                      <div className="flex items-center gap-1">
                        <button onClick={() => openEditChannel(channel)} className="btn btn-ghost btn-xs btn-circle" aria-label={t('common.edit')}>
                          <Pencil className="h-3.5 w-3.5" />
                        </button>
                        <ConfirmDialog title={t('providerDetail.channels.confirmDelete', { name: channel.name })} onConfirm={() => deleteChannelMutation.mutateAsync(channel.id)} okText={t('common.delete')}>
                          <button className="btn btn-ghost btn-xs btn-circle text-error hover:text-error" aria-label={t('common.delete')}>
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
                        <span className="text-base-content/25 text-md">{t('providerDetail.channels.empty')}</span>
                        <button onClick={openAddChannel} className="link link-primary text-md">{t('providerDetail.channels.addFirst')}</button>
                      </div>
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </div>
      )}

      <Modal open={modelModalOpen} onClose={() => setModelModalOpen(false)} title={editingChannelModel ? t('providerDetail.modelModal.editTitle') : t('providerDetail.modelModal.addTitle')}>
        <form onSubmit={handleSaveModel} className="space-y-4">
          {!editingChannelModel && (
            <div className="form-control">
              <label className="label"><span className="label-text">{t('providerDetail.modelModal.modelId')}</span></label>
              <input type="text" value={modelId} onChange={(e) => setModelId(e.target.value)} placeholder={t('providerDetail.modelModal.modelIdPlaceholder')} required className="input input-bordered w-full" />
            </div>
          )}
          <div className="form-control">
            <label className="label"><span className="label-text">{t('providerDetail.modelModal.upstreamModelName')}</span></label>
            <input type="text" value={upstreamModelName} onChange={(e) => setUpstreamModelName(e.target.value)} placeholder={t('providerDetail.modelModal.upstreamModelNamePlaceholder')} className="input input-bordered w-full" />
          </div>
          <div className="form-control">
            <label className="label"><span className="label-text">{t('providerDetail.modelModal.priorityOverride')}</span></label>
            <input type="number" value={priorityOverride} onChange={(e) => setPriorityOverride(e.target.value)} placeholder={t('providerDetail.modelModal.priorityOverridePlaceholder')} className="input input-bordered w-full" />
          </div>
          <div className="form-control">
            <label className="label"><span className="label-text">{t('providerDetail.modelModal.pricingPolicy')}</span></label>
            <Select
              value={pricingPolicyId}
              onChange={(v) => setPricingPolicyId(v as string)}
              options={[{ value: '', label: t('providerDetail.modelModal.noPolicy') }, ...(policies ?? []).map(p => ({ value: p.id, label: p.name }))]}
            />
          </div>
          <div className="form-control">
            <label className="label"><span className="label-text">{t('providerDetail.modelModal.markupRatio')}</span></label>
            <input type="number" value={markupRatio} onChange={(e) => setMarkupRatio(e.target.value)} min={0} step={0.1} className="input input-bordered w-full" />
          </div>
          {editingChannelModel && (
            <div className="flex items-center justify-between">
              <label className="label-text">{t('providerDetail.modelModal.enabled')}</label>
              <Toggle checked={modelEnabled} onChange={setModelEnabled} />
            </div>
          )}
          <Button variant="primary" type="submit" loading={createModelMutation.isPending || updateModelMutation.isPending}>{editingChannelModel ? t('common.save') : t('common.create')}</Button>
        </form>
      </Modal>

      <Modal open={channelModalOpen} onClose={() => setChannelModalOpen(false)} title={editingChannel ? t('providerDetail.channelModal.editTitle', { name: editingChannel.name }) : t('providerDetail.channelModal.addTitle')}>
        <form onSubmit={handleSaveChannel} className="space-y-4">
          <div className="form-control"><label className="label"><span className="label-text">{t('providerDetail.channelModal.name')}</span></label><input type="text" value={channelName} onChange={(e) => setChannelName(e.target.value)} placeholder={t('providerDetail.channelModal.namePlaceholder')} required className="input input-bordered w-full" /></div>
          <div className="form-control"><label className="label"><span className="label-text">{t('providerDetail.channelModal.apiKey')}</span></label><input type="password" value={channelApiKey} onChange={(e) => setChannelApiKey(e.target.value)} placeholder={t('providerDetail.channelModal.apiKeyPlaceholder')} required className="input input-bordered w-full" /></div>
          <div className="form-control"><label className="label"><span className="label-text">{t('providerDetail.channelModal.priority')}</span></label><input type="number" value={channelPriority} onChange={(e) => setChannelPriority(e.target.value)} min={0} className="input input-bordered w-full" /></div>
          {editingChannel && (<div className="flex items-center justify-between"><label className="label-text">{t('providerDetail.channelModal.enabled')}</label><Toggle checked={channelEnabled} onChange={setChannelEnabled} /></div>)}
          <Button variant="primary" type="submit" loading={createChannelMutation.isPending || updateChannelMutation.isPending}>{editingChannel ? t('common.save') : t('common.create')}</Button>
        </form>
      </Modal>
    </div>
  );
}
