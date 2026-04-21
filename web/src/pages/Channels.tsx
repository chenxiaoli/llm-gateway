import { useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useAllChannels, useChannelModels } from '../hooks/useChannels';
import { useProviders } from '../hooks/useProviders';
import { useAllModels } from '../hooks/useModels';
import { createChannel, createChannelModelByChannel } from '../api/providers';
import { Link } from 'react-router-dom';
import { Button } from '../components/ui/Button';
import { Toggle } from '../components/ui/Toggle';
import { Globe, Plus, Radio, Hash, ShieldCheck, ChevronRight, Key, Wifi, Cpu, Search } from 'lucide-react';
import type { Channel, CreateChannelRequest } from '../types';
import { motion, AnimatePresence } from 'framer-motion';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';

// ── Model Panel (outside drawer — fixed left panel) ────────────────────────────
function ModelPanel({
  onClose,
  selectedModelIds,
  upstreamOverrides,
  onToggleModel,
  onUpstreamChange,
}: {
  onClose: () => void;
  selectedModelIds: Set<string>;
  upstreamOverrides: Record<string, string>;
  onToggleModel: (id: string) => void;
  onUpstreamChange: (id: string, v: string) => void;
}) {
  const { data: allModels } = useAllModels();
  const [filter, setFilter] = useState('');

  const filtered = allModels?.filter((m) =>
    m.name.toLowerCase().includes(filter.toLowerCase())
  ) ?? [];

  return (
    <div className="fixed left-0 top-0 bottom-0 w-[200px] bg-base-100 border-r border-base-300 z-[109] flex flex-col shadow-[4px_0_24px_rgba(0,0,0,0.25)]">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-4 border-b border-base-300 shrink-0">
        <div className="flex items-center gap-2">
          <Cpu className="h-4 w-4 text-accent" />
          <span className="text-[13px] font-semibold text-base-content">Models</span>
          {selectedModelIds.size > 0 && (
            <span className="h-4 min-w-[16px] px-1 rounded-full bg-accent text-[10px] font-bold text-white flex items-center justify-center">
              {selectedModelIds.size}
            </span>
          )}
        </div>
        <button
          onClick={onClose}
          className="w-6 h-6 flex items-center justify-center rounded text-base-content/40 hover:text-base-content hover:bg-base-200 transition-all cursor-pointer"
        >
          <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
            <path d="M1 1l8 8M9 1L1 9" />
          </svg>
        </button>
      </div>

      {/* Search */}
      <div className="px-4 py-3 shrink-0 border-b border-base-300/50">
        <div className="relative">
          <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-base-content/30" />
          <input
            type="text"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder="filter models..."
            className="w-full h-8 rounded-lg border border-base-300/60 bg-base-200/50 pl-8 pr-3 text-[12px] text-base-content placeholder:text-base-content/25 focus:outline-none focus:border-accent/50 transition-colors"
          />
        </div>
      </div>

      {/* Model list */}
      <div className="flex-1 overflow-y-auto px-2 py-2 space-y-0.5">
        {filtered.length === 0 && (
          <p className="text-[11px] text-base-content/30 text-center py-6">No models</p>
        )}
        {filtered.map((model) => {
          const checked = selectedModelIds.has(model.id);
          return (
            <label
              key={model.id}
              className={`group flex flex-col gap-1 px-3 py-2.5 rounded-lg cursor-pointer transition-all duration-100 border ${
                checked
                  ? 'border-accent/40 bg-accent/5'
                  : 'border-transparent hover:bg-base-200/50'
              }`}
            >
              <div className="flex items-start gap-2">
                <input
                  type="checkbox"
                  checked={checked}
                  onChange={() => onToggleModel(model.id)}
                  className="mt-0.5 h-3.5 w-3.5 rounded border-base-300 text-accent focus:ring-accent/30 accent-accent cursor-pointer shrink-0"
                />
                <span className={`text-[12px] font-mono leading-tight ${checked ? 'text-base-content/90' : 'text-base-content/60'}`}>
                  {model.name}
                </span>
              </div>
              <AnimatePresence>
                {checked && (
                  <motion.div
                    initial={{ height: 0, opacity: 0 }}
                    animate={{ height: 'auto', opacity: 1 }}
                    exit={{ height: 0, opacity: 0 }}
                    transition={{ duration: 0.12 }}
                    className="overflow-hidden"
                  >
                    <input
                      type="text"
                      value={upstreamOverrides[model.id] ?? ''}
                      onChange={(e) => onUpstreamChange(model.id, e.target.value)}
                      onClick={(e) => e.stopPropagation()}
                      placeholder="upstream name (opt)"
                      className="w-full h-6 rounded border border-base-300 bg-base-200/50 px-2 text-[10px] font-mono text-base-content placeholder:text-base-content/25 focus:outline-none focus:border-accent/60 transition-colors"
                    />
                  </motion.div>
                )}
              </AnimatePresence>
            </label>
          );
        })}
      </div>

      {/* Footer */}
      {selectedModelIds.size > 0 && (
        <div className="px-3 py-3 border-t border-base-300 shrink-0">
          <p className="text-[10px] text-base-content/40 text-center">
            {selectedModelIds.size} model{selectedModelIds.size !== 1 ? 's' : ''} selected
          </p>
        </div>
      )}
    </div>
  );
}

// ── Add Channel Drawer ─────────────────────────────────────────────────────────
function AddChannelDrawer({
  open,
  onClose,
  providers,
  modelPanelOpen,
  onCloseModelPanel,
  selectedModelIds,
  upstreamOverrides,
  onToggleModel,
  onUpstreamChange,
}: {
  open: boolean;
  onClose: () => void;
  providers?: Array<{ id: string; name: string }>;
  modelPanelOpen: boolean;
  onCloseModelPanel: () => void;
  selectedModelIds: Set<string>;
  upstreamOverrides: Record<string, string>;
  onToggleModel: (id: string) => void;
  onUpstreamChange: (id: string, v: string) => void;
}) {
  const queryClient = useQueryClient();
  const [isPending, setIsPending] = useState(false);
  const [providerId, setProviderId] = useState('');
  const [name, setName] = useState('');
  const [apiKey, setApiKey] = useState('');
  const [priority, setPriority] = useState('1');
  const [enabled, setEnabled] = useState(false);

  const reset = () => {
    setProviderId('');
    setName('');
    setApiKey('');
    setPriority('1');
    setEnabled(false);
  };

  const handleClose = () => { reset(); onClose(); onCloseModelPanel(); };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!providerId) { toast.error('Select a provider'); return; }
    setIsPending(true);
    try {
      const input: CreateChannelRequest = {
        provider_id: providerId,
        name,
        api_key: apiKey,
        priority: priority ? parseInt(priority) : 1,
        enabled,
      };
      const channel = await createChannel(input);

      const ids = Array.from(selectedModelIds);
      if (ids.length > 0) {
        const errors: string[] = [];
        await Promise.all(
          ids.map(async (modelId) => {
            try {
              await createChannelModelByChannel(channel.id, {
                model_id: modelId,
                upstream_model_name: upstreamOverrides[modelId] || null,
                enabled,
              });
            } catch {
              errors.push(modelId);
            }
          })
        );
        if (errors.length > 0) {
          toast.warning(`Channel created but ${errors.length} model(s) failed`);
        } else {
          toast.success('Channel created');
        }
      } else {
        toast.success('Channel created');
      }

      queryClient.invalidateQueries({ queryKey: ['channels'] });
      handleClose();
    } catch (err) {
      toast.error(getErrorMessage(err, 'Failed to create channel'));
    } finally {
      setIsPending(false);
    }
  };

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-[110]"
      style={{ background: 'rgba(0, 0, 0, 0.5)' }}
      onClick={handleClose}
    >
      {/* Model panel — outside the drawer overlay */}
      <AnimatePresence>
        {modelPanelOpen && (
          <ModelPanel
            onClose={onCloseModelPanel}
            selectedModelIds={selectedModelIds}
            upstreamOverrides={upstreamOverrides}
            onToggleModel={onToggleModel}
            onUpstreamChange={onUpstreamChange}
          />
        )}
      </AnimatePresence>

      {/* Drawer panel */}
      <div
        onClick={(e) => e.stopPropagation()}
        className={`absolute right-0 top-0 bottom-0 flex flex-col bg-base-100 border-base-300 border-l shadow-[-8px_0_32px_rgba(0,0,0,0.4)] ${
          modelPanelOpen ? 'left-[200px]' : 'left-0'
        } transition-all duration-200`}
        style={{ width: '440px' }}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-5 shrink-0 border-b border-base-300">
          <h3 className="text-[15px] font-semibold text-base-content">Add Channel</h3>
          <button
            onClick={handleClose}
            aria-label="Close"
            className="w-7 h-7 flex items-center justify-center rounded-md bg-transparent cursor-pointer text-base-content/50 hover:text-base-content hover:bg-base-200 transition-all"
          >
            <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
              <path d="M1 1l12 12M13 1L1 13" />
            </svg>
          </button>
        </div>

        {/* Form body */}
        <div className="flex-1 overflow-y-auto px-6 py-5">
          <form onSubmit={handleSubmit} className="space-y-5">
            {/* Provider */}
            <div className="space-y-1.5">
              <label className="text-[11px] font-semibold uppercase tracking-wider text-base-content/50 flex items-center gap-1.5">
                <Globe className="h-3.5 w-3.5" />
                Provider
              </label>
              <select
                value={providerId}
                onChange={(e) => setProviderId(e.target.value)}
                required
                className="w-full h-9 rounded-lg border border-base-300 bg-base-200/50 px-3 text-[13px] text-base-content focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
              >
                <option value="">Select...</option>
                {providers?.map((p) => (
                  <option key={p.id} value={p.id}>{p.name}</option>
                ))}
              </select>
            </div>

            {/* Name */}
            <div className="space-y-1.5">
              <label className="text-[11px] font-semibold uppercase tracking-wider text-base-content/50 flex items-center gap-1.5">
                <Radio className="h-3.5 w-3.5" />
                Name
              </label>
              <div className="relative">
                <input
                  type="text"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  required
                  placeholder="openai-primary"
                  className="w-full h-9 rounded-lg border border-base-300 bg-base-200/50 pl-9 pr-3 text-[13px] font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
                />
              </div>
            </div>

            {/* API Key */}
            <div className="space-y-1.5">
              <label className="text-[11px] font-semibold uppercase tracking-wider text-base-content/50 flex items-center gap-1.5">
                <Key className="h-3.5 w-3.5" />
                API Key
              </label>
              <div className="relative">
                <input
                  type="password"
                  value={apiKey}
                  onChange={(e) => setApiKey(e.target.value)}
                  required
                  placeholder="sk-..."
                  className="w-full h-9 rounded-lg border border-base-300 bg-base-200/50 pl-9 pr-3 text-[13px] font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
                />
                <ShieldCheck className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-base-content/25" />
              </div>
            </div>

            {/* Priority */}
            <div className="space-y-1.5">
              <label className="text-[11px] font-semibold uppercase tracking-wider text-base-content/50 flex items-center gap-1.5">
                <Hash className="h-3.5 w-3.5" />
                Priority
                <span className="text-base-content/20 normal-case font-normal tracking-normal text-[10px]">(lower = higher)</span>
              </label>
              <input
                type="number"
                min="1"
                value={priority}
                onChange={(e) => setPriority(e.target.value)}
                required
                className="w-full h-9 rounded-lg border border-base-300 bg-base-200/50 pl-9 pr-3 text-[13px] font-mono text-base-content focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
              />
            </div>

            {/* Enabled */}
            <div className="flex items-center justify-between">
              <span className="text-[13px] font-medium text-base-content">Enabled</span>
              <Toggle checked={enabled} onChange={setEnabled} />
            </div>

            {/* Selected models badge */}
            {selectedModelIds.size > 0 && (
              <div className="flex items-center gap-2 px-3 py-2 rounded-lg bg-accent/8 border border-accent/20">
                <Cpu className="h-3.5 w-3.5 text-accent/70" />
                <span className="text-[12px] font-medium text-accent/80">
                  {selectedModelIds.size} model{selectedModelIds.size !== 1 ? 's' : ''} selected
                </span>
                <button
                  type="button"
                  onClick={onCloseModelPanel}
                  className="ml-auto text-[10px] text-base-content/40 hover:text-base-content/70 transition-colors cursor-pointer"
                >
                  deselect all
                </button>
              </div>
            )}

            {/* Actions */}
            <div className="flex items-center gap-2 pt-1">
              <Button type="submit" variant="primary" loading={isPending} className="flex-1">
                Create Channel
              </Button>
              <Button type="button" variant="ghost" onClick={handleClose}>
                Cancel
              </Button>
            </div>
          </form>
        </div>

        {/* Footer toolbar — toggle model panel */}
        <div className="shrink-0 border-t border-base-300 px-6 py-3">
          <button
            type="button"
            onClick={modelPanelOpen ? onCloseModelPanel : () => {}}
            className={`flex items-center gap-2 w-full h-9 px-3 rounded-lg text-[12px] font-medium transition-all cursor-pointer border ${
              modelPanelOpen
                ? 'bg-accent/10 border-accent/30 text-accent'
                : 'bg-base-200/50 border-base-300/60 text-base-content/60 hover:bg-base-200 hover:text-base-content/80'
            }`}
          >
            <Cpu className="h-3.5 w-3.5" />
            {modelPanelOpen ? 'Hide models' : 'Select models'}
          </button>
        </div>
      </div>
    </div>
  );
}

// ── Channel Row ────────────────────────────────────────────────────────────────
interface ChannelRowProps {
  channel: Channel;
  providerName: string;
  index: number;
}

function ChannelRow({ channel, providerName, index }: ChannelRowProps) {
  const [expanded, setExpanded] = useState(true);
  const { data: channelModels } = useChannelModels(channel.id);

  return (
    <motion.div
      initial={{ opacity: 0, x: -16 }}
      animate={{ opacity: 1, x: 0 }}
      transition={{ duration: 0.4, delay: index * 0.06, ease: [0.25, 0.1, 0.25, 1] }}
      className="group"
    >
      <div
        className={`relative rounded-xl border overflow-hidden transition-all duration-200 ${
          channel.enabled
            ? 'border-base-300/60 bg-base-100/60 hover:bg-base-100/80 hover:border-accent/25'
            : 'border-base-300/30 bg-base-100/40 hover:bg-base-100/60 hover:border-base-300/50'
        }`}
      >
        <div
          className={`absolute left-0 top-0 bottom-0 w-[3px] transition-all duration-200 ${
            channel.enabled
              ? 'bg-gradient-to-b from-accent via-accent/60 to-accent/20'
              : 'bg-base-300/40'
          }`}
        />

        <div className="px-5 py-4 pl-6">
          <div className="flex items-center gap-4">
            <div
              className={`w-9 h-9 rounded-lg flex items-center justify-center shrink-0 transition-colors duration-200 ${
                channel.enabled
                  ? 'bg-accent/10 text-accent'
                  : 'bg-base-200/60 text-base-content/30'
              }`}
            >
              <Radio className="h-4 w-4" />
            </div>

            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2 mb-0.5">
                <span className="font-mono text-[13px] font-semibold text-base-content/90 truncate">
                  {channel.name}
                </span>
                {channel.enabled && (
                  <div className="flex items-center gap-1 px-1.5 py-0.5 rounded bg-accent/10">
                    <span className="w-1.5 h-1.5 rounded-full bg-accent animate-pulse" />
                    <span className="text-[9px] font-bold uppercase tracking-widest text-accent/80">Live</span>
                  </div>
                )}
              </div>
              <div className="flex items-center gap-1.5">
                <Globe className="h-3 w-3 text-base-content/30" />
                <span className="text-[11px] text-base-content/40 font-medium truncate">{providerName}</span>
              </div>
            </div>

            <div className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-base-200/50 shrink-0">
              <Hash className="h-3 w-3 text-base-content/35" />
              <span className="text-[12px] font-mono font-semibold text-base-content/60">{channel.priority}</span>
            </div>

            <div
              className={`flex items-center gap-1.5 px-2.5 py-1 rounded-full text-[10px] font-semibold tracking-wide uppercase shrink-0 transition-colors ${
                channel.enabled
                  ? 'bg-success/10 text-success'
                  : 'bg-base-200/60 text-base-content/35'
              }`}
            >
              <span
                className={`w-1.5 h-1.5 rounded-full transition-colors ${
                  channel.enabled ? 'bg-success' : 'bg-base-content/25'
                }`}
              />
              {channel.enabled ? 'Active' : 'Disabled'}
            </div>

            <button
              onClick={() => setExpanded(!expanded)}
              className="w-8 h-8 rounded-lg flex items-center justify-center text-base-content/30 hover:text-base-content/60 hover:bg-base-200/60 transition-all duration-150 cursor-pointer shrink-0"
            >
              <ChevronRight
                className={`h-4 w-4 transition-transform duration-200 ${expanded ? 'rotate-90' : ''}`}
              />
            </button>
          </div>

          <AnimatePresence>
            {expanded && (
              <motion.div
                initial={{ height: 0, opacity: 0 }}
                animate={{ height: 'auto', opacity: 1 }}
                exit={{ height: 0, opacity: 0 }}
                transition={{ duration: 0.2 }}
                className="overflow-hidden"
              >
                <div className="pt-4 mt-4 border-t border-base-300/30 space-y-3">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <ShieldCheck className="h-3.5 w-3.5 text-base-content/30" />
                      <span className="text-[11px] text-base-content/40">API Key</span>
                    </div>
                    <span className="font-mono text-[11px] text-base-content/50">
                      {channel.api_key ? '••••••••' + channel.api_key.slice(-6) : '—'}
                    </span>
                  </div>

                  {channelModels && channelModels.length > 0 && (
                    <div>
                      <div className="flex items-center gap-2 mb-2">
                        <Cpu className="h-3.5 w-3.5 text-base-content/30" />
                        <span className="text-[11px] text-base-content/40">
                          {channelModels.length} model{channelModels.length !== 1 ? 's' : ''}
                        </span>
                      </div>
                      <div className="flex flex-wrap gap-1.5">
                        {channelModels.map((cm) => (
                          <Link
                            key={cm.id}
                            to={`/settings/models?channelId=${channel.id}`}
                            className="flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-base-200/50 hover:bg-base-200/80 transition-colors text-[11px] font-mono text-base-content/70 border border-base-300/30"
                          >
                            <Cpu className="h-3 w-3" />
                            {cm.upstream_model_name ?? cm.model_id}
                            {!cm.enabled && (
                              <span className="text-[9px] text-base-content/30 ml-0.5">(off)</span>
                            )}
                          </Link>
                        ))}
                      </div>
                    </div>
                  )}

                  {channelModels && channelModels.length === 0 && (
                    <div className="flex items-center justify-center py-3 text-[11px] text-base-content/30">
                      No models linked
                    </div>
                  )}

                  <div className="flex items-center gap-2 pt-1">
                    <Link
                      to={`/channels/${channel.id}`}
                      className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-base-200/50 hover:bg-base-200/80 transition-colors text-[11px] font-medium text-base-content/70 border border-base-300/30 hover:border-base-300/60"
                    >
                      <Wifi className="h-3 w-3" />
                      Configure
                    </Link>
                    <Link
                      to={`/settings/models?channelId=${channel.id}`}
                      className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-base-200/50 hover:bg-base-200/80 transition-colors text-[11px] font-medium text-base-content/70 border border-base-300/30 hover:border-base-300/60"
                    >
                      <Cpu className="h-3 w-3" />
                      Models
                    </Link>
                  </div>
                </div>
              </motion.div>
            )}
          </AnimatePresence>
        </div>
      </div>
    </motion.div>
  );
}

// ── Channel List Page ───────────────────────────────────────────────────────────
export default function Channels() {
  const { data: channels, isLoading } = useAllChannels();
  const { data: providers } = useProviders();
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [modelPanelOpen, setModelPanelOpen] = useState(false);
  const [selectedModelIds, setSelectedModelIds] = useState<Set<string>>(new Set());
  const [upstreamOverrides, setUpstreamOverrides] = useState<Record<string, string>>({});

  const toggleModel = (id: string) => {
    setSelectedModelIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
        setUpstreamOverrides((o) => {
          const u = { ...o };
          delete u[id];
          return u;
        });
      } else {
        next.add(id);
      }
      return next;
    });
  };

  const handleOpenDrawer = () => {
    setSelectedModelIds(new Set());
    setUpstreamOverrides({});
    setModelPanelOpen(false);
    setDrawerOpen(true);
  };

  const handleCloseModelPanel = () => {
    setModelPanelOpen(false);
  };

  const providerMap = Object.fromEntries((providers ?? []).map((p) => [p.id, p.name]));

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-[20px] font-bold text-base-content tracking-tight">Channels</h1>
          <p className="text-[12px] text-base-content/40 mt-0.5">
            {channels?.length ?? 0} channel{channels?.length !== 1 ? 's' : ''}
          </p>
        </div>
        <Button variant="primary" size="sm" onClick={handleOpenDrawer}>
          <Plus className="h-4 w-4" />
          Add Channel
        </Button>
      </div>

      {/* Channel list */}
      {isLoading ? (
        <div className="space-y-3">
          {[...Array(3)].map((_, i) => (
            <div key={i} className="h-20 rounded-xl bg-base-200/30 animate-pulse" />
          ))}
        </div>
      ) : channels && channels.length > 0 ? (
        <div className="space-y-3">
          {channels.map((channel, index) => (
            <ChannelRow
              key={channel.id}
              channel={channel}
              providerName={providerMap[channel.provider_id] ?? '—'}
              index={index}
            />
          ))}
        </div>
      ) : (
        <div className="flex flex-col items-center justify-center py-20 text-center">
          <div className="w-12 h-12 rounded-2xl bg-base-200/50 flex items-center justify-center mb-4">
            <Radio className="h-5 w-5 text-base-content/20" />
          </div>
          <p className="text-[14px] font-medium text-base-content/50 mb-1">No channels yet</p>
          <p className="text-[12px] text-base-content/30 mb-6">Add your first channel</p>
          <Button variant="primary" size="sm" onClick={handleOpenDrawer}>
            <Plus className="h-4 w-4" />
            Add Channel
          </Button>
        </div>
      )}

      {/* Add Channel Drawer + Model Panel */}
      <AddChannelDrawer
        open={drawerOpen}
        onClose={() => { setDrawerOpen(false); setModelPanelOpen(false); }}
        providers={providers?.map((p) => ({ id: p.id, name: p.name }))}
        modelPanelOpen={modelPanelOpen}
        onCloseModelPanel={handleCloseModelPanel}
        selectedModelIds={selectedModelIds}
        upstreamOverrides={upstreamOverrides}
        onToggleModel={toggleModel}
        onUpstreamChange={(id, v) =>
          setUpstreamOverrides((prev) => ({ ...prev, [id]: v }))
        }
      />
    </div>
  );
}
