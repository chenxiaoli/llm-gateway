import { useState, useEffect } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import i18n from '../i18n';
import { useAllChannels, useTestChannel, useToggleChannel } from '../hooks/useChannels';
import { useProviders } from '../hooks/useProviders';
import { useAllModels } from '../hooks/useModels';
import { createChannel } from '../api/providers';
import { Link } from 'react-router-dom';
import { Button } from '../components/ui/Button';
import { Drawer } from '../components/ui/Drawer';
import { Toggle } from '../components/ui/Toggle';
import { Globe, Plus, Radio, Hash, ShieldCheck, Key, Wifi, Cpu, Search, X, Clock, Scale, Zap, Check, Loader2 } from 'lucide-react';
import type { Channel, CreateChannelRequest, TimeSlot } from '../types';
import { motion, AnimatePresence } from 'framer-motion';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';

// ── Availability check (mirrors backend is_available_now) ─────────────────────
function isAvailableNow(slots: TimeSlot[] | null | undefined): boolean {
  if (!slots || slots.length === 0) return true;
  const now = new Date();
  const day = now.toLocaleDateString('en-US', { weekday: 'short', timeZone: 'UTC' }).toLowerCase();
  const nowMinutes = now.getUTCHours() * 60 + now.getUTCMinutes();
  return slots.some((slot) => {
    if (!slot.days.includes(day)) return false;
    const [sh, sm] = slot.start.split(':').map(Number);
    const [eh, em] = slot.end.split(':').map(Number);
    const start = (sh || 0) * 60 + (sm || 0);
    const end = (eh || 0) * 60 + (em || 0);
    return nowMinutes >= start && nowMinutes < end;
  });
}

// ── Searchable Model Multi-Select ─────────────────────────────────────────────
function ModelMultiSelect({
  selected,
  onToggle,
  onRemove,
}: {
  selected: Array<{ id: string; name: string }>;
  onToggle: (id: string) => void;
  onRemove: (id: string) => void;
}) {
  const { data: allModels } = useAllModels();
  const [query, setQuery] = useState('');
  const [open, setOpen] = useState(false);
  const { t } = useTranslation();

  const filtered = (allModels ?? []).filter(
    (m) =>
      m.name.toLowerCase().includes(query.toLowerCase()) &&
      !selected.some((s) => s.id === m.id)
  );

  return (
    <div className="space-y-2">
      <label className="text-base font-semibold uppercase tracking-wider text-base-content/50 flex items-center gap-1.5">
        <Cpu className="h-3.5 w-3.5" />
        {t('channels.modelSelect.label')}
        {selected.length > 0 && (
          <span className="normal-case font-normal tracking-normal text-base-content/30">
            {t('channels.modelSelect.selected', { count: selected.length })}
          </span>
        )}
      </label>

      {/* Selected chips */}
      {selected.length > 0 && (
        <div className="flex flex-wrap gap-1.5 p-2.5 rounded-lg border border-base-300 bg-base-200/30 min-h-[44px]">
          {selected.map((m) => (
            <span
              key={m.id}
              className="inline-flex items-center gap-1 pl-2 pr-1.5 py-1 rounded-md bg-accent/10 border border-accent/25 text-base font-mono text-accent/80"
            >
              {m.name}
              <button
                type="button"
                onClick={() => onRemove(m.id)}
                className="w-3.5 h-3.5 rounded flex items-center justify-center hover:bg-accent/20 transition-colors cursor-pointer"
              >
                <X className="h-2.5 w-2.5" />
              </button>
            </span>
          ))}
        </div>
      )}

      {/* Search input */}
      <div className="relative">
        <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-base-content/30 pointer-events-none" />
        <input
          type="text"
          value={query}
          onChange={(e) => { setQuery(e.target.value); setOpen(true); }}
          onFocus={() => setOpen(true)}
          placeholder={t('channels.modelSelect.searchPlaceholder')}
          className="w-full h-9 rounded-lg border border-base-300 bg-base-200/50 pl-9 pr-3 text-md text-base-content placeholder:text-base-content/25 focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
        />
      </div>

      {/* Dropdown */}
      <AnimatePresence>
        {open && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: 'auto', opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.12 }}
            className="overflow-hidden"
          >
            <div
              className="border border-base-300 rounded-lg bg-base-100 shadow-lg overflow-y-auto"
              style={{ maxHeight: '180px' }}
            >
              {filtered.length === 0 ? (
                <p className="text-md text-base-content/30 text-center py-4">
                  {query ? t('channels.modelSelect.noMatch') : t('channels.modelSelect.noModels')}
                </p>
              ) : (
                filtered.map((m) => (
                  <button
                    key={m.id}
                    type="button"
                    onClick={() => { onToggle(m.id); setQuery(''); setOpen(false); }}
                    className="w-full flex items-center gap-2.5 px-3 py-2 text-left hover:bg-base-200/60 transition-colors cursor-pointer border-b border-base-300/30 last:border-0"
                  >
                    <Cpu className="h-3.5 w-3.5 text-base-content/30 shrink-0" />
                    <span className="text-md font-mono text-base-content/80 truncate">{m.name}</span>
                  </button>
                ))
              )}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

// ── Add Channel Drawer ────────────────────────────────────────────────────────
function AddChannelDrawer({
  open,
  onClose,
  providers,
}: {
  open: boolean;
  onClose: () => void;
  providers?: Array<{ id: string; name: string }>;
}) {
  const queryClient = useQueryClient();
  const { data: allModels } = useAllModels();
  const { t } = useTranslation();
  const [isPending, setIsPending] = useState(false);
  const [providerId, setProviderId] = useState('');
  const [name, setName] = useState('');
  const [apiKey, setApiKey] = useState('');
  const [priority, setPriority] = useState('1');
  const [weight, setWeight] = useState('');
  const [enabled, setEnabled] = useState(false);
  const [group, setGroup] = useState('');
  const [selectedModelIds, setSelectedModelIds] = useState<Set<string>>(new Set());

  const reset = () => {
    setProviderId('');
    setName('');
    setApiKey('');
    setPriority('1');
    setWeight('');
    setEnabled(false);
    setGroup('');
    setSelectedModelIds(new Set());
  };

  const handleClose = () => { reset(); onClose(); };

  const selectedModels = (allModels ?? []).filter((m) => selectedModelIds.has(m.id));

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!providerId) { toast.error(i18n.t('channels.addDrawer.selectProviderError')); return; }
    setIsPending(true);
    try {
      const models = selectedModelIds.size > 0
        ? Array.from(selectedModelIds).map((id) => ({ model_id: id, enabled }))
        : undefined;
      const input: CreateChannelRequest = {
        provider_id: providerId,
        name,
        api_key: apiKey,
        priority: priority ? parseInt(priority) : 1,
        weight: weight ? parseInt(weight) : null,
        enabled,
        models,
        group: group || undefined,
      };
      await createChannel(input);
      queryClient.invalidateQueries({ queryKey: ['channels'] });
      toast.success(i18n.t('toasts.channelCreated'));
      handleClose();
    } catch (err) {
      toast.error(getErrorMessage(err, i18n.t('toasts.channelCreateFailed')));
    } finally {
      setIsPending(false);
    }
  };

  return (
    <Drawer open={open} onClose={handleClose} title={t('channels.addDrawer.title')} width={440}>
      <form onSubmit={handleSubmit} className="space-y-6">
        {/* Provider */}
        <div className="space-y-1.5">
          <label className="text-base font-semibold uppercase tracking-wider text-base-content/50 flex items-center gap-1.5">
            <Globe className="h-3.5 w-3.5" />
            {t('channels.addDrawer.provider')}
          </label>
          <select
            value={providerId}
            onChange={(e) => setProviderId(e.target.value)}
            required
            className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-md text-base-content focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
          >
            <option value="">{t('channels.addDrawer.selectProvider')}</option>
            {providers?.map((p) => (
              <option key={p.id} value={p.id}>{p.name}</option>
            ))}
          </select>
        </div>

        {/* Channel name */}
        <div className="space-y-1.5">
          <label className="text-base font-semibold uppercase tracking-wider text-base-content/50 flex items-center gap-1.5">
            <Radio className="h-3.5 w-3.5" />
            {t('channels.addDrawer.channelName')}
          </label>
          <div className="relative">
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              required
              placeholder={t('channels.addDrawer.channelNamePlaceholder')}
              className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 pl-9 pr-3 text-md font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
            />
          </div>
        </div>

        {/* Base URL */}
        <div className="space-y-1.5">
          <label className="text-base font-semibold uppercase tracking-wider text-base-content/50 flex items-center gap-1.5">
            <Key className="h-3.5 w-3.5" />
            {t('channels.addDrawer.apiKey')}
          </label>
          <div className="relative">
            <input
              type="password"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              required
              placeholder="sk-..."
              className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 pl-9 pr-3 text-md font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
            />
            <ShieldCheck className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-base-content/25" />
          </div>
        </div>

        {/* Priority */}
        <div className="space-y-1.5">
          <label className="text-base font-semibold uppercase tracking-wider text-base-content/50 flex items-center gap-1.5">
            <Hash className="h-3.5 w-3.5" />
            {t('channels.addDrawer.priority')}
            <span className="text-base-content/20 normal-case font-normal tracking-normal text-base">{t('channels.addDrawer.priorityHint')}</span>
          </label>
          <div className="relative">
            <input
              type="number"
              min="1"
              value={priority}
              onChange={(e) => setPriority(e.target.value)}
              required
              className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 pl-9 pr-3 text-md font-mono text-base-content focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
            />
          </div>
        </div>

        {/* Weight */}
        <div className="space-y-1.5">
          <label className="text-base font-semibold uppercase tracking-wider text-base-content/50 flex items-center gap-1.5">
            <Scale className="h-3.5 w-3.5" />
            {t('channels.addDrawer.weight')}
            <span className="text-base-content/20 normal-case font-normal tracking-normal text-base">{t('channels.addDrawer.weightHint')}</span>
          </label>
          <div className="relative">
            <input
              type="number"
              min="0"
              value={weight}
              onChange={(e) => setWeight(e.target.value)}
              placeholder="100"
              className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 pl-9 pr-3 text-md font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
            />
          </div>
        </div>

        {/* Models */}
        <ModelMultiSelect
          selected={selectedModels}
          onToggle={(id) => {
            setSelectedModelIds((prev) => {
              const next = new Set(prev);
              next.has(id) ? next.delete(id) : next.add(id);
              return next;
            });
          }}
          onRemove={(id) => {
            setSelectedModelIds((prev) => {
              const next = new Set(prev);
              next.delete(id);
              return next;
            });
          }}
        />

        {/* Enabled */}
        <div className="flex items-center justify-between">
          <div className="space-y-0.5">
            <span className="text-md font-medium text-base-content">{t('channels.addDrawer.enabled')}</span>
            <p className="text-base text-base-content/40">{t('channels.addDrawer.enabledHint')}</p>
          </div>
          <Toggle checked={enabled} onChange={setEnabled} />
        </div>

        <div>
          <label className="label"><span className="label-text font-medium">{t('channelDetail.editModal.group')}</span></label>
          <input
            type="text"
            value={group}
            onChange={(e) => setGroup(e.target.value)}
            placeholder={t('channelDetail.editModal.groupPlaceholder')}
            className="input input-bordered w-full"
          />
        </div>

        {/* Actions */}
        <div className="flex items-center gap-2 pt-2">
          <Button
            type="submit"
            variant="primary"
            loading={isPending}
            className="flex-1"
          >
            {t('channels.addDrawer.createChannel')}
          </Button>
          <Button type="button" variant="ghost" onClick={handleClose}>
            {t('common.cancel')}
          </Button>
        </div>
      </form>
    </Drawer>
  );
}

// ── Channel Row ────────────────────────────────────────────────────────────────
interface ChannelRowProps {
  channel: Channel;
  providerName: string;
  index: number;
}

function ChannelRow({ channel, providerName, index }: ChannelRowProps) {
  const testMutation = useTestChannel();
  const toggleMutation = useToggleChannel();
  const { t } = useTranslation();
  const [testStatus, setTestStatus] = useState<'idle' | 'loading' | 'success' | 'error'>('idle');
  const [testDetail, setTestDetail] = useState<{ latency_ms: number; model: string; error: string | null; response_data: string | null } | null>(null);

  useEffect(() => {
    if (testStatus === 'success' || testStatus === 'error') {
      const timer = setTimeout(() => setTestStatus('idle'), 5000);
      return () => clearTimeout(timer);
    }
  }, [testStatus]);

  const handleTest = () => {
    setTestStatus('loading');
    testMutation.mutate({ id: channel.id }, {
      onSuccess: (result) => {
        setTestDetail({ latency_ms: result.latency_ms, model: result.model, error: result.error, response_data: result.response_data });
        if (result.success) {
          setTestStatus('success');
          toast.success(i18n.t('channels.row.testOk', { latency: result.latency_ms, model: result.model }));
        } else {
          setTestStatus('error');
          toast.error(i18n.t('channels.row.testFailed', { error: result.error ?? i18n.t('channels.row.unknownError') }));
        }
      },
      onError: (err) => {
        setTestStatus('error');
        toast.error(getErrorMessage(err, i18n.t('channels.row.testFailedShort')));
      },
    });
  };

  const channelModels = channel.models ?? [];

  return (
    <motion.div
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.35, delay: index * 0.06, ease: [0.25, 0.1, 0.25, 1] }}
    >
      <div
        className={`group relative flex items-center gap-4 px-5 py-3.5 rounded-xl border transition-all duration-200 ${
          channel.enabled
            ? 'border-base-300/60 bg-base-100/50 hover:bg-base-100/80 hover:border-accent/30'
            : 'border-base-300/30 bg-base-100/30 hover:bg-base-100/50 hover:border-base-300/50 opacity-80'
        }`}
      >
        {/* Status indicator — left dot */}
        <div
          className={`w-1.5 h-1.5 rounded-full shrink-0 ${
            channel.enabled ? 'bg-success' : 'bg-base-content/20'
          }`}
        />

        {/* Icon */}
        <div
          className={`w-8 h-8 rounded-lg flex items-center justify-center shrink-0 ${
            channel.enabled
              ? 'bg-accent/10 text-accent'
              : 'bg-base-200/50 text-base-content/30'
          }`}
        >
          <Radio className="h-3.5 w-3.5" />
        </div>

        {/* Name + provider */}
        <div className="min-w-0 shrink-0" style={{ width: '160px' }}>
          <p className="font-mono text-md font-semibold text-base-content/90 truncate leading-tight">
            {channel.name}
          </p>
          <p className="text-base text-base-content/40 truncate mt-0.5">{providerName}</p>
        </div>

        {/* Priority */}
        <div className="shrink-0">
          <div className="flex items-center gap-1 px-2 py-1 rounded bg-base-200/50">
            <Hash className="h-3 w-3 text-base-content/35" />
            <span className="text-md font-mono font-semibold text-base-content/55">{channel.priority}</span>
          </div>
        </div>

        <div className="shrink-0">
          <div className="flex items-center gap-1 px-2 py-1 rounded bg-accent/5 border border-accent/10">
            <Scale className="h-3 w-3 text-accent/50" />
            <span className="text-md font-mono font-semibold text-accent/60">{channel.weight ?? 100}</span>
          </div>
        </div>

        {channel.group && (
          <div className="shrink-0">
            <span className="inline-flex items-center px-2 py-1 rounded bg-info/10 text-info text-xs font-medium">
              {channel.group}
            </span>
          </div>
        )}

        {/* Models */}
        <div className="flex-1 min-w-0">
          {channelModels && channelModels.length > 0 ? (
            <div className="flex flex-wrap gap-1">
              {channelModels.slice(0, 6).map((cm) => (
                <div
                  key={cm.id}
                  className={`badge badge-neutral badge-outline ${!cm.enabled ? 'opacity-40' : ''}`}
                  title={cm.upstream_model_name ? `upstream: ${cm.upstream_model_name}` : undefined}
                >
                  {cm.model_name}
                </div>
              ))}
              {channelModels.length > 6 && (
                <div className="badge badge-neutral badge-outline opacity-40">
                  +{channelModels.length - 6}
                </div>
              )}
            </div>
          ) : (
            <span className="text-base text-base-content/25">{t('channels.row.noModels')}</span>
          )}
        </div>

        {/* Available Hours */}
        {channel.available_hours && channel.available_hours.length > 0 ? (() => {
          const available = isAvailableNow(channel.available_hours);
          return (
            <div className={`shrink-0 flex flex-col gap-0.5 px-2 py-1 rounded ${available ? 'bg-success/5 border border-success/15' : 'bg-base-200/50 border border-base-300/30'}`}>
              <div className="flex items-center gap-1.5">
                <div className={`w-1.5 h-1.5 rounded-full shrink-0 ${available ? 'bg-success' : 'bg-base-content/20'}`} />
                <span className={`text-sm font-semibold ${available ? 'text-success/80' : 'text-base-content/35'}`}>
                  {available ? t('channels.row.available') : t('channels.row.outsideHours')}
                </span>
              </div>
              {channel.available_hours.map((slot, i) => (
                <div key={i} className="flex items-center gap-1.5">
                  <Clock className="h-3 w-3 text-base-content/35 shrink-0" />
                  <span className="text-md font-mono text-base-content/50 whitespace-nowrap">{slot.start}–{slot.end}</span>
                  <div className="flex gap-0.5">
                    {slot.days.map(d => (
                      <span key={d} className="text-sm font-medium text-primary/70 bg-primary/8 px-1 rounded">{d.slice(0, 3)}</span>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          );
        })() : (
          <div className="shrink-0 flex items-center gap-1.5 px-2 py-1 rounded">
            <div className="w-1.5 h-1.5 rounded-full bg-success shrink-0" />
            <span className="text-md font-mono text-base-content/25">24/7</span>
          </div>
        )}

        {/* Enable toggle */}
        <Toggle
          checked={channel.enabled}
          onChange={(enabled) => toggleMutation.mutate({ id: channel.id, enabled })}
        />

        {/* Quick actions */}
        <div className="flex items-center gap-1 shrink-0 opacity-0 group-hover:opacity-100 transition-opacity duration-150">
          <button
            onClick={handleTest}
            disabled={testStatus === 'loading'}
            className={`inline-flex items-center gap-1 px-2.5 py-1.5 rounded-lg text-md font-medium transition-all duration-100 border border-transparent ${
              testStatus === 'success'
                ? 'text-success hover:text-success'
                : testStatus === 'error'
                ? 'text-error hover:text-error'
                : 'text-base-content/50 hover:text-base-content/80 hover:bg-base-200/70 hover:border-base-300/40'
            }`}
          >
            {testStatus === 'loading' && <Loader2 className="h-3 w-3 animate-spin" />}
            {testStatus === 'success' && <Check className="h-3 w-3" />}
            {testStatus === 'error' && <X className="h-3 w-3" />}
            {testStatus === 'idle' && <Zap className="h-3 w-3" />}
            {testStatus === 'loading' ? t('channels.row.testing') : testStatus === 'success' ? t('channels.row.ok') : testStatus === 'error' ? t('channels.row.fail') : t('channels.row.test')}
          </button>
          {testDetail && (testStatus === 'success' || testStatus === 'error') && (
            <button
              onClick={() => setTestDetail(null)}
              className="inline-flex items-center gap-1 px-2.5 py-1.5 rounded-lg text-md font-medium text-base-content/50 hover:text-base-content/80 hover:bg-base-200/70 hover:border-base-300/40 transition-all duration-100 border border-transparent"
            >
              {t('channels.row.viewDetail')}
            </button>
          )}
          <Link
            to={`/admin/channels/${channel.id}`}
            className="inline-flex items-center gap-1 px-2.5 py-1.5 rounded-lg text-md font-medium text-base-content/50 hover:text-base-content/80 hover:bg-base-200/70 transition-all duration-100 border border-transparent hover:border-base-300/40"
          >
            <Wifi className="h-3 w-3" />
            {t('channels.row.configure')}
          </Link>
        </div>

        {/* Test Result Detail Modal */}
        {testDetail && (testStatus === 'success' || testStatus === 'error') && (
          <div className="mt-3 p-3 rounded-xl border border-base-300/40 bg-base-100">
            <div className="text-[10px] font-mono font-semibold uppercase tracking-[0.18em] text-base-content/25 mb-3">
              {t('channels.row.testDetailTitle').toUpperCase()}
            </div>
            <div className="space-y-4">
              <div className="grid grid-cols-2 gap-3">
                <div className="rounded-lg bg-base-200/60 p-3">
                  <div className="text-[10px] font-semibold uppercase tracking-wider text-base-content/40 mb-1">{t('channels.row.testStatus')}</div>
                  <div className={`font-medium ${testStatus === 'success' ? 'text-success' : 'text-error'}`}>
                    {testStatus === 'success' ? t('channels.row.ok') : t('channels.row.fail')}
                  </div>
                </div>
                <div className="rounded-lg bg-base-200/60 p-3">
                  <div className="text-[10px] font-semibold uppercase tracking-wider text-base-content/40 mb-1">{t('channels.row.testLatency')}</div>
                  <div className="mono text-[13px]">{testDetail.latency_ms}ms</div>
                </div>
                <div className="rounded-lg bg-base-200/60 p-3">
                  <div className="text-[10px] font-semibold uppercase tracking-wider text-base-content/40 mb-1">{t('channels.row.testModel')}</div>
                  <div className="mono text-[13px]">{testDetail.model}</div>
                </div>
                {testDetail.error && (
                  <div className="rounded-lg bg-base-200/60 p-3">
                    <div className="text-[10px] font-semibold uppercase tracking-wider text-base-content/40 mb-1">{t('channels.row.testError')}</div>
                    <div className="mono text-[13px] text-error truncate">{testDetail.error}</div>
                  </div>
                )}
              </div>
              {testDetail.response_data && (
                <div>
                  <div className="text-[10px] font-semibold uppercase tracking-wider text-base-content/40 mb-2">{t('channels.row.testResponse')}</div>
                  <pre className="rounded-lg bg-base-200/60 p-4 text-xs mono overflow-auto max-h-48 whitespace-pre-wrap break-all">
                    {(() => {
                      try {
                        return JSON.stringify(JSON.parse(testDetail.response_data!), null, 2);
                      } catch {
                        return testDetail.response_data;
                      }
                    })()}
                  </pre>
                </div>
              )}
            </div>
          </div>
        )}
      </div>
    </motion.div>
  );
}

// ── Empty State ───────────────────────────────────────────────────────────────
function EmptyState({ onAddClick }: { onAddClick: () => void }) {
  const { t } = useTranslation();
  return (
    <motion.div
      initial={{ opacity: 0, y: 24 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.5 }}
      className="flex flex-col items-center justify-center py-28 px-4"
    >
      {/* Decorative background */}
      <div className="relative mb-8">
        <div className="w-20 h-20 rounded-2xl bg-base-200/50 flex items-center justify-center">
          <Wifi className="h-9 w-9 text-base-content/15" />
        </div>
        {/* Decorative rings */}
        <div className="absolute inset-0 rounded-2xl border border-dashed border-base-300/30 -m-3" />
        <div className="absolute -top-1 -right-1 w-6 h-6 rounded-full bg-accent/10 border border-accent/20 flex items-center justify-center">
          <Plus className="h-3.5 w-3.5 text-accent" />
        </div>
      </div>

      <h3 className="text-lg font-semibold text-base-content/50 mb-1.5">{t('channels.empty.title')}</h3>
      <p className="text-md text-base-content/25 mb-8 text-center max-w-xs leading-relaxed">
        {t('channels.empty.description')}
      </p>

      <button
        onClick={onAddClick}
        className="inline-flex items-center gap-2 px-5 py-2.5 rounded-xl bg-accent/10 hover:bg-accent/15 border border-accent/20 text-accent text-md font-semibold transition-all duration-200 cursor-pointer"
      >
        <Plus className="h-4 w-4" />
        {t('channels.empty.addFirst')}
      </button>
    </motion.div>
  );
}

// ── Stats Bar ─────────────────────────────────────────────────────────────────
function StatsBar({ channels }: { channels: Channel[] }) {
  const { t } = useTranslation();
  const total = channels.length;
  const active = channels.filter(c => c.enabled).length;
  const disabled = total - active;

  return (
    <motion.div
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.4, delay: 0.1 }}
      className="grid grid-cols-3 gap-3 mb-7"
    >
      <div className="rounded-xl border border-base-300/50 bg-base-100/50 px-4 py-3">
        <div className="text-base uppercase tracking-widest text-base-content/30 font-semibold mb-1">{t('channels.stats.totalChannels')}</div>
        <div className="text-xl font-bold text-base-content font-mono">{total}</div>
      </div>
      <div className="rounded-xl border border-success/20 bg-success/5 px-4 py-3">
        <div className="text-base uppercase tracking-widest text-success/70 font-semibold mb-1">{t('channels.stats.active')}</div>
        <div className="text-xl font-bold text-success font-mono">{active}</div>
      </div>
      <div className="rounded-xl border border-base-300/50 bg-base-100/50 px-4 py-3">
        <div className="text-base uppercase tracking-widest text-base-content/30 font-semibold mb-1">{t('channels.stats.disabled')}</div>
        <div className="text-xl font-bold text-base-content/40 font-mono">{disabled}</div>
      </div>
    </motion.div>
  );
}

// ── Main Page ─────────────────────────────────────────────────────────────────
export default function Channels() {
  const { data: channels, isLoading } = useAllChannels();
  const { data: providers } = useProviders();
  const { t } = useTranslation();
  const [isAdding, setIsAdding] = useState(false);

  const getProviderName = (providerId: string) =>
    providers?.find(p => p.id === providerId)?.name ?? providerId;

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-24">
        <div className="flex flex-col items-center gap-3">
          <div className="w-8 h-8 rounded-full border-2 border-accent/30 border-t-accent animate-spin" />
          <span className="text-md text-base-content/35 font-medium">{t('channels.loading')}</span>
        </div>
      </div>
    );
  }

  const totalChannels = channels?.length ?? 0;
  const activeChannels = channels?.filter(c => c.enabled).length ?? 0;

  return (
    <div className="px-6 pb-8">
      {/* Page header */}
      <motion.div
        initial={{ opacity: 0, y: -8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.4 }}
        className="mb-6 flex items-start justify-between pt-8"
      >
        <div>
          <div className="flex items-center gap-3 mb-1">
            <h1 className="text-xl font-bold tracking-tight text-base-content">{t('channels.title')}</h1>
            {totalChannels > 0 && (
              <span className="inline-flex items-center px-2 py-0.5 rounded-full text-base font-bold uppercase tracking-widest bg-base-200/70 text-base-content/40 border border-base-300/50">
                {totalChannels}
              </span>
            )}
          </div>
          <p className="text-md text-base-content/35">
            {totalChannels === 0
              ? t('channels.description')
              : t('channels.descriptionCount', { active: activeChannels, disabled: totalChannels - activeChannels })}
          </p>
        </div>

        <AnimatePresence>
          {totalChannels > 0 && (
            <motion.div
              initial={{ opacity: 0, scale: 0.92 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={{ opacity: 0, scale: 0.92 }}
              transition={{ duration: 0.2 }}
            >
              <Button
                icon={<Plus className="h-4 w-4" />}
                size="sm"
                onClick={() => setIsAdding(true)}
              >
                {t('channels.addChannel')}
              </Button>
            </motion.div>
          )}
        </AnimatePresence>
      </motion.div>

      {/* Stats strip */}
      {totalChannels > 0 && <StatsBar channels={channels!} />}

      {/* Channel list or empty state */}
      {totalChannels === 0 ? (
        <EmptyState onAddClick={() => setIsAdding(true)} />
      ) : (
        <motion.div
          initial="hidden"
          animate="visible"
          variants={{ hidden: {}, visible: { transition: { staggerChildren: 0.06 } } }}
          className="space-y-3"
        >
          {channels!.map((channel, i) => (
            <ChannelRow
              key={channel.id}
              channel={channel}
              providerName={getProviderName(channel.provider_id)}
              index={i}
            />
          ))}
        </motion.div>
      )}

      <AddChannelDrawer
        open={isAdding}
        onClose={() => setIsAdding(false)}
        providers={providers?.map(p => ({ id: p.id, name: p.name }))}
      />
    </div>
  );
}
