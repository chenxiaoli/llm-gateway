import { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import i18n from 'i18next';
import { ArrowLeft, Pencil, Trash2, KeyRound, Hash, Plus, Building2, LinkIcon, Power, Eye, EyeOff, Clock, Scale, Zap } from 'lucide-react';
import { useChannel, useUpdateChannel, useDeleteChannel, useChannelModels, useCreateChannelModel, useDeleteChannelModel, useUpdateChannelModel, useUpdateChannelApiKey, useProviderModels, useTestChannel } from '../hooks/useChannels';
import { useProviders } from '../hooks/useProviders';
import { useAllModels } from '../hooks/useModels';
import { getErrorMessage } from '../api/client';
import { usePricingPolicies } from '../hooks/usePricingPolicies';
import { Button } from '../components/ui/Button';
import { Modal } from '../components/ui/Modal';
import { Toggle } from '../components/ui/Toggle';
import { ConfirmDialog } from '../components/ui/ConfirmDialog';
import type { UpdateChannelRequest, ChannelModel, TimeSlot } from '../types';
import { utcToLocalTime, localToUtcTime, utcDayToLocalDay, localDayToUtcDay, getBrowserTimezone, getTimezoneLabel } from '../lib/timezone';

export default function ChannelDetail() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { t } = useTranslation();
  const { data: channel, isLoading } = useChannel(id!);
  const { data: providers } = useProviders();
  const { data: channelModels, isLoading: modelsLoading } = useChannelModels(channel?.id || '');
  const { data: allModels } = useAllModels();
  const { data: providerModels } = useProviderModels(channel?.provider_id ?? '');
  const { data: policies } = usePricingPolicies();
  const updateMutation = useUpdateChannel(id!);
  const deleteMutation = useDeleteChannel(id!);
  const createModelMutation = useCreateChannelModel(channel?.id || '');
  const deleteModelMutation = useDeleteChannelModel(channel?.id || '');
  const updateModelMutation = useUpdateChannelModel(channel?.id || '');
  const updateApiKeyMutation = useUpdateChannelApiKey(channel?.id || '');
  const testMutation = useTestChannel();
  const [testStatus, setTestStatus] = useState<'idle' | 'loading' | 'success' | 'error'>('idle');

  const [isEditing, setIsEditing] = useState(false);
  const [endpointTestStatus, setEndpointTestStatus] = useState<Record<string, 'idle' | 'loading' | 'success' | 'error'>>({});
  const [channelName, setChannelName] = useState('');
  const [channelPriority, setChannelPriority] = useState('0');
  const [channelWeight, setChannelWeight] = useState('');
  const [channelEnabled, setChannelEnabled] = useState(false);
  const [channelGroup, setChannelGroup] = useState('');
  const [revealKey, setRevealKey] = useState(false);

  const [isUpdatingApiKey, setIsUpdatingApiKey] = useState(false);
  const [newApiKey, setNewApiKey] = useState('');

  const [isAddingModel, setIsAddingModel] = useState(false);
  const [selectedModel, setSelectedModel] = useState('');
  const [upstreamModelName, setUpstreamModelName] = useState('');
  const [pricingPolicyId, setPricingPolicyId] = useState('');
  const [markupRatio, setMarkupRatio] = useState('1.0');
  const [modelEnabled, setModelEnabled] = useState(true);
  const [editingModel, setEditingModel] = useState<ChannelModel | null>(null);

  const [editingHours, setEditingHours] = useState(false);
  const [hoursSlots, setHoursSlots] = useState<TimeSlot[]>([]);

  useEffect(() => {
    if (channel) {
      setChannelName(channel.name);
      setChannelPriority(String(channel.priority));
      setChannelWeight(channel.weight != null ? String(channel.weight) : '');
      setChannelEnabled(channel.enabled);
      setChannelGroup(channel.group ?? '');
      setHoursSlots(channel.available_hours ?? []);
    }
  }, [channel]);

  useEffect(() => {
    if (testStatus === 'success' || testStatus === 'error') {
      const timer = setTimeout(() => setTestStatus('idle'), 3000);
      return () => clearTimeout(timer);
    }
  }, [testStatus]);

  useEffect(() => {
    const hasDone = Object.values(endpointTestStatus).some((s) => s === 'success' || s === 'error');
    if (hasDone) {
      const timer = setTimeout(() => setEndpointTestStatus({}), 3000);
      return () => clearTimeout(timer);
    }
  }, [endpointTestStatus]);

  const handleTest = () => {
    setTestStatus('loading');
    testMutation.mutate({ id: channel!.id }, {
      onSuccess: (results) => {
        const result = results[0];
        if (result && !result.error) {
          setTestStatus('success');
          toast.success(i18n.t('channels.row.testOk', { latency: result.latency_ms, model: result.model }));
        } else {
          setTestStatus('error');
          toast.error(i18n.t('channels.row.testFailed', { error: result?.error ?? i18n.t('channels.row.unknownError') }));
        }
      },
      onError: (err) => {
        setTestStatus('error');
        toast.error(getErrorMessage(err, i18n.t('channels.row.testFailedShort')));
      },
    });
  };

  const handleTestEndpoint = (endpointKey: string) => {
    setEndpointTestStatus((prev) => ({ ...prev, [endpointKey]: 'loading' }));
    testMutation.mutate({ id: channel!.id, endpointKey }, {
      onSuccess: (results) => {
        const result = results[0];
        if (result && !result.error) {
          setEndpointTestStatus((prev) => ({ ...prev, [endpointKey]: 'success' }));
          toast.success(i18n.t('channels.row.testOk', { latency: result.latency_ms, model: result.model }));
        } else {
          setEndpointTestStatus((prev) => ({ ...prev, [endpointKey]: 'error' }));
          toast.error(i18n.t('channels.row.testFailed', { error: result?.error ?? i18n.t('channels.row.unknownError') }));
        }
      },
      onError: (err) => {
        setEndpointTestStatus((prev) => ({ ...prev, [endpointKey]: 'error' }));
        toast.error(getErrorMessage(err, i18n.t('channels.row.testFailedShort')));
      },
    });
  };

  const browserTz = getBrowserTimezone();
  const tzLabel = getTimezoneLabel(browserTz);

  const localSlots = (slots: TimeSlot[] | null | undefined): TimeSlot[] | null => {
    if (!slots || slots.length === 0) return slots ?? null;
    return slots.map(s => ({
      days: s.days.map(d => utcDayToLocalDay(d, s.start, browserTz)),
      start: utcToLocalTime(s.start, browserTz),
      end: utcToLocalTime(s.end, browserTz),
    }));
  };

  const toUtcSlots = (local: TimeSlot[]): TimeSlot[] =>
    local.map(s => ({
      days: s.days.map(d => localDayToUtcDay(d, s.start, browserTz)),
      start: localToUtcTime(s.start, browserTz),
      end: localToUtcTime(s.end, browserTz),
    }));

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
        <div className="text-base-content/40 mb-4">{t('channelDetail.channelNotFound')}</div>
        <Button variant="secondary" onClick={() => navigate('/admin/channels')}>
          {t('channelDetail.backToChannels')}
        </Button>
      </div>
    );
  }

  const provider = providers?.find(p => p.id === channel.provider_id);

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    const input: UpdateChannelRequest = {
      name: channelName,
      priority: Number(channelPriority),
      weight: channelWeight ? Number(channelWeight) : null,
      enabled: channelEnabled,
      group: channelGroup || null,
    };
    await updateMutation.mutateAsync({ id: channel.id, input });
    setIsEditing(false);
  };

  const handleDelete = async () => {
    await deleteMutation.mutateAsync(channel.id);
    navigate('/admin/channels');
  };

  const handleCancelEdit = () => {
    setChannelName(channel.name);
    setChannelPriority(String(channel.priority));
    setChannelWeight(channel.weight != null ? String(channel.weight) : '');
    setChannelEnabled(channel.enabled);
    setChannelGroup(channel.group ?? '');
    setIsEditing(false);
  };

  const handleUpdateApiKey = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newApiKey.trim()) return;
    await updateApiKeyMutation.mutateAsync(newApiKey.trim());
    setNewApiKey('');
    setIsUpdatingApiKey(false);
  };

  return (
    <div>
      <Button variant="ghost" icon={<ArrowLeft className="h-4 w-4" />} onClick={() => navigate('/admin/channels')} className="mb-4">
        {t('channelDetail.backToChannels')}
      </Button>

      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold">{t('channelDetail.pageTitle', { name: channel.name })}</h1>
          <p className="text-sm text-base-content/40 mt-1">
            {t('channelDetail.providerLabel', { name: provider?.name ?? channel.provider_id })}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button
            variant="secondary"
            icon={<Zap className="h-4 w-4" />}
            onClick={handleTest}
            loading={testStatus === 'loading'}
            disabled={testStatus === 'loading'}
          >
            {testStatus === 'loading' ? t('channels.row.testing') : testStatus === 'success' ? t('channels.row.ok') : testStatus === 'error' ? t('channels.row.fail') : t('channels.row.test')}
          </Button>
          <Button variant="secondary" icon={<Pencil className="h-4 w-4" />} onClick={() => setIsEditing(true)}>
            {t('common.edit')}
          </Button>
          <ConfirmDialog title={t('channelDetail.deleteChannel', { name: channel.name })} onConfirm={handleDelete} okText={t('common.delete')}>
            <Button variant="danger" icon={<Trash2 className="h-4 w-4" />}>{t('common.delete')}</Button>
          </ConfirmDialog>
        </div>
      </div>

      <div className="grid gap-6 max-w-2xl">
        {/* Status Card */}
        <div className="bg-base-100 rounded-box p-5 shadow-sm">
          <h2 className="text-sm font-semibold text-base-content/60 mb-4">{t('channelDetail.status')}</h2>
          <div className="flex items-center gap-3">
            <span className={`inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full text-sm font-medium ${
              channel.enabled
                ? 'bg-success/10 text-success'
                : 'bg-base-300/50 text-base-content/40'
            }`}>
              <span className={`w-2 h-2 rounded-full ${channel.enabled ? 'bg-success' : 'bg-base-content/30'}`} />
              {channel.enabled ? t('channelDetail.active') : t('channelDetail.disabled')}
            </span>
          </div>
        </div>

        {/* Details Card */}
        <div className="bg-base-100 rounded-box p-5 shadow-sm">
          <h2 className="text-sm font-semibold text-base-content/60 mb-4">{t('channelDetail.configuration')}</h2>
          <div className="space-y-4">
            <div className="flex items-center gap-3">
              <div className="w-8 h-8 bg-primary/10 flex items-center justify-center shrink-0">
                <KeyRound className="h-4 w-4 text-primary" />
              </div>
              <div className="flex-1 min-w-0">
                <div className="text-xs text-base-content/40 uppercase tracking-wider">{t('channelDetail.apiKey')}</div>
                <div className="flex items-center gap-2 mt-0.5">
                  {channel.api_key ? (
                    revealKey ? (
                      <span className="text-sm font-mono text-primary bg-primary/5 px-2 py-0.5 rounded-none select-all">
                        {channel.api_key}
                      </span>
                    ) : (
                      <span className="text-sm font-mono text-base-content/60 tracking-widest">
                        {'•'.repeat(Math.min(channel.api_key.length, 32))}
                      </span>
                    )
                  ) : (
                    <span className="text-sm text-base-content/30 italic">{t('channelDetail.notSet')}</span>
                  )}
                  {channel.api_key && (
                    <button
                      onClick={() => setRevealKey(!revealKey)}
                      className="text-base-content/30 hover:text-base-content/60 transition-colors duration-150 cursor-pointer"
                      title={revealKey ? t('channelDetail.hide') : t('channelDetail.reveal')}
                    >
                      {revealKey
                        ? <EyeOff className="h-3.5 w-3.5" />
                        : <Eye className="h-3.5 w-3.5" />
                      }
                    </button>
                  )}
                  <button
                    onClick={() => { setNewApiKey(''); setIsUpdatingApiKey(true); }}
                    className="text-xs text-primary hover:text-primary/80 font-medium underline underline-offset-2 transition-colors duration-150 cursor-pointer"
                  >
                    {t('channelDetail.update')}
                  </button>
                </div>
              </div>
            </div>

            <div className="flex items-center gap-3">
              <div className="w-8 h-8 rounded-lg bg-primary/10 flex items-center justify-center shrink-0">
                <Hash className="h-4 w-4 text-primary" />
              </div>
              <div className="flex-1">
                <div className="text-xs text-base-content/40 uppercase tracking-wider">{t('channelDetail.priority')}</div>
                <div className="text-sm font-mono text-base-content/80">
                  {channel.priority}
                </div>
              </div>
            </div>

            <div className="flex items-center gap-3">
              <div className="w-8 h-8 rounded-lg bg-accent/10 flex items-center justify-center shrink-0">
                <Scale className="h-4 w-4 text-accent" />
              </div>
              <div className="flex-1">
                <div className="text-xs text-base-content/40 uppercase tracking-wider">{t('channelDetail.weight')}</div>
                <div className="text-sm font-mono text-base-content/80">
                  {channel.weight ?? 100}
                </div>
              </div>
            </div>

            {channel.group && (
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 rounded-lg bg-info/10 flex items-center justify-center shrink-0">
                  <Hash className="h-4 w-4 text-info" />
                </div>
                <div className="flex-1">
                  <div className="text-xs text-base-content/40 uppercase tracking-wider">{t('channelDetail.group')}</div>
                  <div className="text-sm font-mono text-base-content/80">{channel.group}</div>
                </div>
              </div>
            )}
          </div>
        </div>

        {/* Provider Card */}
        {provider && (
          <div className="bg-base-100 rounded-box p-5 shadow-sm">
            <h2 className="text-sm font-semibold text-base-content/60 mb-4">{t('channelDetail.provider')}</h2>
            <div className="space-y-4">
              <div className="flex items-center gap-3">
                <div className="w-8 h-8 rounded-lg bg-secondary/10 flex items-center justify-center shrink-0">
                  <Building2 className="h-4 w-4 text-secondary" />
                </div>
                <div className="flex-1 min-w-0">
                  <div className="text-xs text-base-content/40 uppercase tracking-wider">{t('channelDetail.name')}</div>
                  <div className="text-sm font-medium">{provider.name}</div>
                </div>
              </div>

              {provider?.endpoints && (
                <div className="flex items-start gap-3">
                  <div className="w-8 h-8 rounded-lg bg-secondary/10 flex items-center justify-center shrink-0 mt-0.5">
                    <LinkIcon className="h-4 w-4 text-secondary" />
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="text-xs text-base-content/40 uppercase tracking-wider mb-2">{t('channelDetail.endpoints')}</div>
                    <div className="grid grid-cols-2 gap-2">
                      {Object.entries(provider.endpoints).map(([key, value]) => (
                        <div key={key} className="bg-base-200/50 rounded px-2 py-1.5 flex items-center gap-2">
                          <div className="flex-1 min-w-0">
                            <div className="text-xs text-secondary font-medium capitalize">{key}</div>
                            <div className="text-xs font-mono text-base-content/60 truncate" title={value}>
                              {value}
                            </div>
                          </div>
                          <button
                            className="btn btn-ghost btn-xs text-primary shrink-0"
                            disabled={endpointTestStatus[key] === 'loading'}
                            onClick={() => handleTestEndpoint(key)}
                          >
                            {endpointTestStatus[key] === 'loading'
                              ? <span className="loading loading-spinner loading-xs" />
                              : endpointTestStatus[key] === 'success'
                                ? <span className="text-success text-xs">{t('channels.row.ok')}</span>
                                : endpointTestStatus[key] === 'error'
                                  ? <span className="text-error text-xs">{t('channels.row.fail')}</span>
                                  : <Zap className="h-3 w-3" />}
                          </button>
                        </div>
                      ))}
                    </div>
                  </div>
                </div>
              )}

              <div className="flex items-center gap-3">
                <div className="w-8 h-8 rounded-lg bg-secondary/10 flex items-center justify-center shrink-0">
                  <Power className="h-4 w-4 text-secondary" />
                </div>
                <div className="flex-1">
                  <div className="text-xs text-base-content/40 uppercase tracking-wider">{t('channelDetail.status')}</div>
                  <span className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded text-xs font-medium ${
                    provider.enabled
                      ? 'bg-success/10 text-success'
                      : 'bg-base-300/50 text-base-content/40'
                  }`}>
                    {provider.enabled ? t('channelDetail.enabled') : t('channelDetail.disabled')}
                  </span>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Available Hours Card */}
        <div className="bg-base-100 rounded-box p-5 shadow-sm">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-sm font-semibold text-base-content/60">{t('channelDetail.availableHours')} <span className="text-xs font-normal text-base-content/30">{tzLabel}</span></h2>
            <Button variant="secondary" size="sm" icon={<Pencil className="h-4 w-4" />} onClick={() => { const local = localSlots(channel.available_hours ?? []); setHoursSlots(local ?? []); setEditingHours(true); }}>
              {t('common.edit')}
            </Button>
          </div>
          {(() => {
            const displayed = localSlots(channel.available_hours);
            return displayed && displayed.length > 0 ? (
            <div className="space-y-2">
              {displayed.map((slot, i) => (
                <div key={i} className="flex items-center gap-3 p-3 bg-base-200/50 rounded-lg">
                  <Clock className="h-4 w-4 text-primary shrink-0" />
                  <div className="flex-1">
                    <span className="font-mono text-sm text-base-content/80">{slot.start} – {slot.end}</span>
                  </div>
                  <div className="flex gap-1 flex-wrap justify-end">
                    {slot.days.map(d => (
                      <span key={d} className="px-2 py-0.5 bg-primary/10 text-primary/80 rounded text-xs font-medium capitalize">{d}</span>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className="text-sm text-base-content/30 italic">{t('channelDetail.alwaysAvailable')}</div>
          );
          })()}
        </div>

        {/* Metadata Card */}
        <div className="bg-base-100 rounded-box p-5 shadow-sm">
          <h2 className="text-sm font-semibold text-base-content/60 mb-4">{t('channelDetail.metadata')}</h2>
          <div className="grid grid-cols-2 gap-4 text-sm">
            <div>
              <div className="text-xs text-base-content/40 uppercase tracking-wider mb-1">{t('channelDetail.id')}</div>
              <div className="font-mono text-base-content/60 text-xs truncate" title={channel.id}>
                {channel.id}
              </div>
            </div>
            <div>
              <div className="text-xs text-base-content/40 uppercase tracking-wider mb-1">{t('channelDetail.created')}</div>
              <div className="text-base-content/60">
                {new Date(channel.created_at).toLocaleDateString()}
              </div>
            </div>
            <div>
              <div className="text-xs text-base-content/40 uppercase tracking-wider mb-1">{t('channelDetail.updated')}</div>
              <div className="text-base-content/60">
                {new Date(channel.updated_at).toLocaleDateString()}
              </div>
            </div>
          </div>
        </div>

        {/* Channel Models Card */}
        <div className="bg-base-100 rounded-box p-5 shadow-sm">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-sm font-semibold text-base-content/60">{t('channelDetail.channelModels')}</h2>
            <Button variant="secondary" size="sm" icon={<Plus className="h-4 w-4" />} onClick={() => setIsAddingModel(true)}>
              {t('channelDetail.addModel')}
            </Button>
          </div>

          {modelsLoading ? (
            <div className="flex items-center justify-center py-8">
              <span className="loading loading-spinner loading-sm" />
            </div>
          ) : channelModels && channelModels.length > 0 ? (
            <div className="space-y-2">
              {channelModels.map((cm) => {
                const model = allModels?.find(m => m.id === cm.model_id);
                const policy = policies?.find(p => p.id === cm.pricing_policy_id);
                return (
                <div key={cm.id} className="p-4 rounded-lg bg-base-200/50 space-y-3">
                  {/* Header row: model name + status + actions */}
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2 min-w-0 flex-1">
                      <div className="font-mono text-sm font-semibold truncate text-base-content/90">
                        {model?.name ?? cm.model_id}
                      </div>
                      <span className={`inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[10px] font-semibold shrink-0 ${
                        cm.enabled
                          ? 'bg-success/10 text-success'
                          : 'bg-base-300/50 text-base-content/40'
                      }`}>
                        <span className={`w-1.5 h-1.5 rounded-full ${cm.enabled ? 'bg-success' : 'bg-base-content/30'}`} />
                        {cm.enabled ? t('channelDetail.active') : t('channelDetail.disabled')}
                      </span>
                    </div>
                    <div className="flex items-center gap-1 shrink-0">
                      <Button variant="ghost" size="sm" icon={<Pencil className="h-4 w-4" />} onClick={() => setEditingModel(cm)} className="cursor-pointer" />
                      <ConfirmDialog title={t('channelDetail.removeModel', { name: model?.name ?? cm.upstream_model_name })} onConfirm={() => deleteModelMutation.mutateAsync(cm.id)} okText={t('common.remove')}>
                        <Button variant="ghost" size="sm" icon={<Trash2 className="h-4 w-4" />} />
                      </ConfirmDialog>
                    </div>
                  </div>

                  {/* Detail rows */}
                  <div className="grid grid-cols-2 gap-x-6 gap-y-1.5 text-xs">
                    <div className="flex items-start gap-2">
                      <span className="text-base-content/30 w-20 shrink-0">{t('channelDetail.upstream')}</span>
                      <span className="font-mono text-base-content/60 truncate" title={cm.upstream_model_name ?? ''}>
                        {cm.upstream_model_name ?? <span className="italic text-base-content/25">{t('channelDetail.notSet')}</span>}
                      </span>
                    </div>
                    <div className="flex items-start gap-2">
                      <span className="text-base-content/30 w-20 shrink-0">{t('channelDetail.pricing')}</span>
                      {policy ? (
                        <span className="inline-flex items-center px-1.5 py-0.5 rounded bg-primary/10 text-primary/80 font-medium border border-primary/15">
                          {policy.name}
                        </span>
                      ) : (
                        <span className="text-base-content/25 italic">{t('channelDetail.noPolicy')}</span>
                      )}
                    </div>
                    <div className="flex items-start gap-2">
                      <span className="text-base-content/30 w-20 shrink-0">{t('channelDetail.markup')}</span>
                      <span className="font-mono text-base-content/60">
                        ×{cm.markup_ratio.toFixed(2)}
                      </span>
                    </div>
                    <div className="flex items-start gap-2">
                      <span className="text-base-content/30 w-20 shrink-0">{t('channelDetail.priorityOverride')}</span>
                      <span className="font-mono text-base-content/60">
                        {cm.priority_override != null ? cm.priority_override : <span className="text-base-content/25 italic">{t('channelDetail.defaultPriority')}</span>}
                      </span>
                    </div>
                  </div>
                </div>
              );
              })}
            </div>
          ) : (
            <div className="text-center py-8 text-base-content/40">
              <p className="text-sm">{t('channelDetail.noModels')}</p>
              <Button variant="ghost" size="sm" className="mt-2" onClick={() => setIsAddingModel(true)}>
                {t('channelDetail.addFirstModel')}
              </Button>
            </div>
          )}
        </div>
      </div>

      {/* Edit Modal */}
      <Modal open={isEditing} onClose={handleCancelEdit} title={t('channelDetail.editModal.title')}>
        <form onSubmit={handleSave} className="space-y-4">
          <div className="form-control">
            <label className="label">
              <span className="label-text">{t('channelDetail.editModal.name')}</span>
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
              <span className="label-text">{t('channelDetail.editModal.priority')}</span>
            </label>
            <input
              type="number"
              value={channelPriority}
              onChange={(e) => setChannelPriority(e.target.value)}
              min={0}
              className="input input-bordered w-full"
            />
          </div>

          <div className="form-control">
            <label className="label">
              <span className="label-text">{t('channelDetail.editModal.weight')}</span>
              <span className="label-text-alt text-base-content/40">{t('channelDetail.editModal.weightHint')}</span>
            </label>
            <input
              type="number"
              value={channelWeight}
              onChange={(e) => setChannelWeight(e.target.value)}
              min={0}
              placeholder="100"
              className="input input-bordered w-full"
            />
          </div>

          <div>
            <label className="label"><span className="label-text font-medium">{t('channelDetail.editModal.group')}</span></label>
            <input
              type="text"
              value={channelGroup}
              onChange={(e) => setChannelGroup(e.target.value)}
              placeholder={t('channelDetail.editModal.groupPlaceholder')}
              className="input input-bordered w-full"
            />
          </div>

          <div className="flex items-center justify-between">
            <span className="label-text">{t('channelDetail.editModal.enabled')}</span>
            <Toggle checked={channelEnabled} onChange={setChannelEnabled} />
          </div>

          <div className="flex gap-2 pt-2">
            <Button variant="primary" type="submit" loading={updateMutation.isPending}>
              {t('common.save')}
            </Button>
            <Button variant="ghost" type="button" onClick={handleCancelEdit}>
              {t('common.cancel')}
            </Button>
          </div>
        </form>
      </Modal>

      {/* Add Model Modal */}
      <Modal open={isAddingModel} onClose={() => setIsAddingModel(false)} title={t('channelDetail.addModelModal.title')}>
        <form onSubmit={async (e) => {
          e.preventDefault();
          if (!selectedModel) return;
          await createModelMutation.mutateAsync({
            model_id: selectedModel,
            upstream_model_name: upstreamModelName || undefined,
            pricing_policy_id: pricingPolicyId || undefined,
            markup_ratio: Number(markupRatio),
            enabled: modelEnabled,
          });
          setIsAddingModel(false);
          setSelectedModel('');
          setUpstreamModelName('');
          setPricingPolicyId('');
          setMarkupRatio('1.0');
          setModelEnabled(true);
        }} className="space-y-4">
          <div className="form-control">
            <label className="label">
              <span className="label-text">{t('channelDetail.addModelModal.selectModel')}</span>
            </label>
            <select
              value={selectedModel}
              onChange={(e) => {
                setSelectedModel(e.target.value);
                const pm = (providerModels && providerModels.length > 0 ? providerModels : null)
                  ?.find(m => m.model_id === e.target.value);
                if (pm?.upstream_name) setUpstreamModelName(pm.upstream_name);
              }}
              required
              className="select select-bordered w-full"
            >
              <option value="">{t('channelDetail.addModelModal.selectPlaceholder')}</option>
              {(Array.isArray(providerModels) && providerModels.length > 0
                ? providerModels
                    .filter(m => !channelModels?.some(cm => cm.model_id === m.model_id))
                    .map(m => ({ id: m.model_id, name: m.model_name }))
                : allModels
                    ?.filter(m => !channelModels?.some(cm => cm.model_id === m.id))
                    .map(m => ({ id: m.id, name: m.name }))
                    ?? []
              ).map((model) => (
                <option key={model.id} value={model.id}>
                  {model.name}
                </option>
              ))}
            </select>
          </div>

          <div className="form-control">
            <label className="label">
              <span className="label-text">{t('channelDetail.addModelModal.upstreamModelName')}</span>
            </label>
            <input
              type="text"
              value={upstreamModelName}
              onChange={(e) => setUpstreamModelName(e.target.value)}
              placeholder={t('channelDetail.addModelModal.upstreamPlaceholder')}
              className="input input-bordered w-full"
            />
            <label className="label">
              <span className="label-text-alt">{t('channelDetail.addModelModal.upstreamHint')}</span>
            </label>
          </div>

          <div className="form-control">
            <label className="label">
              <span className="label-text">{t('channelDetail.addModelModal.pricingPolicy')}</span>
            </label>
            <select
              value={pricingPolicyId}
              onChange={(e) => setPricingPolicyId(e.target.value)}
              className="select select-bordered w-full"
            >
              <option value="">{t('channelDetail.addModelModal.noPolicy')}</option>
              {policies?.map((p) => (
                <option key={p.id} value={p.id}>{p.name}</option>
              ))}
            </select>
          </div>

          <div className="form-control">
            <label className="label">
              <span className="label-text">{t('channelDetail.addModelModal.markupRatio')}</span>
            </label>
            <input
              type="number"
              value={markupRatio}
              onChange={(e) => setMarkupRatio(e.target.value)}
              min={0}
              step={0.1}
              className="input input-bordered w-full"
            />
            <label className="label">
              <span className="label-text-alt">{t('channelDetail.addModelModal.markupHint')}</span>
            </label>
          </div>

          <div className="flex items-center justify-between">
            <span className="label-text">{t('channelDetail.addModelModal.enabled')}</span>
            <Toggle checked={modelEnabled} onChange={setModelEnabled} />
          </div>

          <div className="flex gap-2 pt-2">
            <Button variant="primary" type="submit" loading={createModelMutation.isPending}>
              {t('channelDetail.addModel')}
            </Button>
            <Button variant="ghost" type="button" onClick={() => setIsAddingModel(false)}>
              {t('common.cancel')}
            </Button>
          </div>
        </form>
      </Modal>

      {/* Edit Channel Model Modal */}
      <Modal open={editingModel !== null} onClose={() => setEditingModel(null)} title={t('channelDetail.editModelModal.title', { name: editingModel?.upstream_model_name ?? 'Model' })}>
        <form onSubmit={async (e) => {
          e.preventDefault();
          if (!editingModel) return;
          await updateModelMutation.mutateAsync({
            id: editingModel.id,
            input: {
              upstream_model_name: (document.getElementById('edit-upstream') as HTMLInputElement).value || undefined,
              pricing_policy_id: (document.getElementById('edit-policy') as HTMLSelectElement).value || undefined,
              markup_ratio: Number((document.getElementById('edit-markup') as HTMLInputElement).value),
              priority_override: (document.getElementById('edit-priority') as HTMLInputElement).value ? Number((document.getElementById('edit-priority') as HTMLInputElement).value) : undefined,
              enabled: (document.getElementById('edit-enabled') as HTMLInputElement).checked,
            },
          });
          setEditingModel(null);
        }} className="space-y-4">
          <div className="form-control">
            <label className="label">
              <span className="label-text">{t('channelDetail.editModelModal.upstreamModelName')}</span>
            </label>
            <input
              id="edit-upstream"
              type="text"
              defaultValue={editingModel?.upstream_model_name ?? ''}
              placeholder={t('channelDetail.addModelModal.upstreamPlaceholder')}
              className="input input-bordered w-full"
            />
          </div>

          <div className="form-control">
            <label className="label">
              <span className="label-text">{t('channelDetail.editModelModal.pricingPolicy')}</span>
            </label>
            <select
              id="edit-policy"
              defaultValue={editingModel?.pricing_policy_id ?? ''}
              className="select select-bordered w-full"
            >
              <option value="">{t('channelDetail.editModelModal.noPolicy')}</option>
              {policies?.map((p) => (
                <option key={p.id} value={p.id}>{p.name}</option>
              ))}
            </select>
          </div>

          <div className="form-control">
            <label className="label">
              <span className="label-text">{t('channelDetail.editModelModal.markupRatio')}</span>
            </label>
            <input
              id="edit-markup"
              type="number"
              defaultValue={editingModel?.markup_ratio ?? 1.0}
              min={0}
              step={0.1}
              className="input input-bordered w-full"
            />
            <label className="label">
              <span className="label-text-alt">{t('channelDetail.editModelModal.markupHint')}</span>
            </label>
          </div>

          <div className="form-control">
            <label className="label">
              <span className="label-text">{t('channelDetail.editModelModal.priorityOverride')}</span>
            </label>
            <input
              id="edit-priority"
              type="number"
              defaultValue={editingModel?.priority_override ?? ''}
              placeholder={t('channelDetail.editModelModal.priorityPlaceholder')}
              className="input input-bordered w-full"
            />
          </div>

          <div className="flex items-center justify-between">
            <span className="label-text">{t('channelDetail.editModelModal.enabled')}</span>
            <input
              id="edit-enabled"
              type="checkbox"
              className="toggle toggle-primary"
              defaultChecked={editingModel?.enabled ?? true}
            />
          </div>

          <div className="flex gap-2 pt-2">
            <Button variant="primary" type="submit" loading={updateModelMutation.isPending}>
              {t('common.save')}
            </Button>
            <Button variant="ghost" type="button" onClick={() => setEditingModel(null)}>
              {t('common.cancel')}
            </Button>
          </div>
        </form>
      </Modal>

      {/* Update API Key Modal */}
      <Modal open={isUpdatingApiKey} onClose={() => setIsUpdatingApiKey(false)} title={t('channelDetail.updateApiKeyModal.title')}>
        <form onSubmit={handleUpdateApiKey} className="space-y-4">
          <div className="bg-warning/10 border border-warning/20 rounded-box px-4 py-3">
            <p className="text-sm text-warning">
              {t('channelDetail.updateApiKeyModal.warning')}
            </p>
          </div>

          <div className="form-control">
            <label className="label">
              <span className="label-text">{t('channelDetail.updateApiKeyModal.newApiKey')}</span>
            </label>
            <input
              type="password"
              value={newApiKey}
              onChange={(e) => setNewApiKey(e.target.value)}
              placeholder={t('channelDetail.updateApiKeyModal.placeholder')}
              required
              autoFocus
              className="input input-bordered w-full font-mono"
              autoComplete="new-password"
            />
          </div>

          <div className="flex gap-2 pt-2">
            <Button variant="primary" type="submit" loading={updateApiKeyMutation.isPending}>
              {t('channelDetail.updateApiKeyModal.updateKey')}
            </Button>
            <Button variant="ghost" type="button" onClick={() => setIsUpdatingApiKey(false)}>
              {t('common.cancel')}
            </Button>
          </div>
        </form>
      </Modal>

      {/* Edit Available Hours Modal */}
      <Modal open={editingHours} onClose={() => setEditingHours(false)} title={t('channelDetail.editHoursModal.title')}>
        <form onSubmit={async (e) => {
          e.preventDefault();
          await updateMutation.mutateAsync({
            id: channel.id,
            input: { available_hours: toUtcSlots(hoursSlots) },
          });
          setEditingHours(false);
        }} className="space-y-4">
          <div className="bg-base-200/50 rounded-box px-4 py-3 text-sm text-base-content/60">
            {t('channelDetail.editHoursModal.description')}
          </div>

          {hoursSlots.map((slot, i) => (
            <div key={i} className="p-4 bg-base-200/30 rounded-lg space-y-3">
              <div className="flex items-center justify-between">
                <span className="text-xs font-semibold text-base-content/50">{t('channelDetail.editHoursModal.slot', { number: i + 1 })}</span>
                <button type="button" onClick={() => setHoursSlots(hoursSlots.filter((_, j) => j !== i))} className="text-base-content/30 hover:text-error text-xs cursor-pointer">{t('channelDetail.editHoursModal.remove')}</button>
              </div>
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="text-xs text-base-content/40 uppercase tracking-wider">{t('channelDetail.editHoursModal.start')}</label>
                  <input type="time" value={slot.start} onChange={e => {
                    const next = [...hoursSlots]; next[i] = { ...next[i], start: e.target.value }; setHoursSlots(next);
                  }} className="input input-bordered w-full text-sm" />
                </div>
                <div>
                  <label className="text-xs text-base-content/40 uppercase tracking-wider">{t('channelDetail.editHoursModal.end')}</label>
                  <input type="time" value={slot.end} onChange={e => {
                    const next = [...hoursSlots]; next[i] = { ...next[i], end: e.target.value }; setHoursSlots(next);
                  }} className="input input-bordered w-full text-sm" />
                </div>
              </div>
              <div>
                <label className="text-xs text-base-content/40 uppercase tracking-wider mb-1 block">{t('channelDetail.editHoursModal.days')}</label>
                <div className="flex flex-wrap gap-1.5">
                  {['mon','tue','wed','thu','fri','sat','sun'].map(d => (
                    <button key={d} type="button" onClick={() => {
                      const next = [...hoursSlots];
                      const days = next[i].days.includes(d) ? next[i].days.filter(x => x !== d) : [...next[i].days, d];
                      next[i] = { ...next[i], days };
                      setHoursSlots(next);
                    }} className={`px-2.5 py-1 rounded text-xs font-medium cursor-pointer transition-colors ${
                      slot.days.includes(d) ? 'bg-primary/15 text-primary border border-primary/25' : 'bg-base-200 text-base-content/40 border border-transparent'
                    }`}>
                      {d.charAt(0).toUpperCase() + d.slice(1)}
                    </button>
                  ))}
                </div>
              </div>
            </div>
          ))}

          <Button variant="ghost" size="sm" type="button" onClick={() => setHoursSlots([...hoursSlots, { days: ['mon','tue','wed','thu','fri'], start: utcToLocalTime('09:00', browserTz), end: utcToLocalTime('17:00', browserTz) }])} className="w-full border border-dashed border-base-content/15">
            {t('channelDetail.editHoursModal.addSlot')}
          </Button>

          <div className="flex gap-2 pt-2">
            <Button variant="primary" type="submit" loading={updateMutation.isPending}>
              {t('common.save')}
            </Button>
            <Button variant="ghost" type="button" onClick={() => setEditingHours(false)}>
              {t('common.cancel')}
            </Button>
          </div>
        </form>
      </Modal>
    </div>
  );
}