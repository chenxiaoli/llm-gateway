import { useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useAllChannels, useChannelModels } from '../hooks/useChannels';
import { useProviders } from '../hooks/useProviders';
import { createChannel } from '../api/providers';
import { Link } from 'react-router-dom';
import { Button } from '../components/ui/Button';
import { Drawer } from '../components/ui/Drawer';
import { Toggle } from '../components/ui/Toggle';
import { Globe, Plus, Radio, Hash, ShieldCheck, ChevronRight, Key, Wifi, Cpu } from 'lucide-react';
import type { Channel, CreateChannelRequest } from '../types';
import { motion, AnimatePresence } from 'framer-motion';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';

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

  const handleClose = () => { reset(); onClose(); };

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
      await createChannel(input);
      queryClient.invalidateQueries({ queryKey: ['channels'] });
      toast.success('Channel created');
      handleClose();
    } catch (err) {
      toast.error(getErrorMessage(err, 'Failed to create channel'));
    } finally {
      setIsPending(false);
    }
  };

  return (
    <Drawer open={open} onClose={handleClose} title="Add Channel" width={440}>
      <form onSubmit={handleSubmit} className="space-y-6">
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
            className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-[13px] text-base-content focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
          >
            <option value="">Select a provider...</option>
            {providers?.map((p) => (
              <option key={p.id} value={p.id}>{p.name}</option>
            ))}
          </select>
        </div>

        {/* Channel name */}
        <div className="space-y-1.5">
          <label className="text-[11px] font-semibold uppercase tracking-wider text-base-content/50 flex items-center gap-1.5">
            <Radio className="h-3.5 w-3.5" />
            Channel Name
          </label>
          <div className="relative">
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              required
              placeholder="e.g. openai-primary, anthropic-failover"
              className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 pl-9 pr-3 text-[13px] font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
            />
          </div>
        </div>

        {/* Base URL */}
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
              className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 pl-9 pr-3 text-[13px] font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
            />
            <ShieldCheck className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-base-content/25" />
          </div>
        </div>

        {/* Priority */}
        <div className="space-y-1.5">
          <label className="text-[11px] font-semibold uppercase tracking-wider text-base-content/50 flex items-center gap-1.5">
            <Hash className="h-3.5 w-3.5" />
            Priority
            <span className="text-base-content/20 normal-case font-normal tracking-normal text-[10px]">(lower = higher priority)</span>
          </label>
          <div className="relative">
            <input
              type="number"
              min="1"
              value={priority}
              onChange={(e) => setPriority(e.target.value)}
              required
              className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 pl-9 pr-3 text-[13px] font-mono text-base-content focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
            />
          </div>
        </div>

        {/* Enabled */}
        <div className="flex items-center justify-between">
          <div className="space-y-0.5">
            <span className="text-[13px] font-medium text-base-content">Enabled</span>
            <p className="text-[11px] text-base-content/40">Channels must be enabled to receive traffic</p>
          </div>
          <Toggle checked={enabled} onChange={setEnabled} />
        </div>

        {/* Actions */}
        <div className="flex items-center gap-2 pt-2">
          <Button
            type="submit"
            variant="primary"
            loading={isPending}
            className="flex-1"
          >
            Create Channel
          </Button>
          <Button type="button" variant="ghost" onClick={handleClose}>
            Cancel
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
  const [expanded, setExpanded] = useState(true); // 默认展开
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
        {/* Left accent bar */}
        <div
          className={`absolute left-0 top-0 bottom-0 w-[3px] transition-all duration-200 ${
            channel.enabled
              ? 'bg-gradient-to-b from-accent via-accent/60 to-accent/20'
              : 'bg-base-300/40'
          }`}
        />

        <div className="px-5 py-4 pl-6">
          {/* Main row */}
          <div className="flex items-center gap-4">
            {/* Channel icon */}
            <div
              className={`w-9 h-9 rounded-lg flex items-center justify-center shrink-0 transition-colors duration-200 ${
                channel.enabled
                  ? 'bg-accent/10 text-accent'
                  : 'bg-base-200/60 text-base-content/30'
              }`}
            >
              <Radio className="h-4 w-4" />
            </div>

            {/* Name & provider */}
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

            {/* Priority badge */}
            <div className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-base-200/50 shrink-0">
              <Hash className="h-3 w-3 text-base-content/35" />
              <span className="text-[12px] font-mono font-semibold text-base-content/60">{channel.priority}</span>
            </div>

            {/* Status */}
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

            {/* Expand / Actions */}
            <button
              onClick={() => setExpanded(!expanded)}
              className="w-8 h-8 rounded-lg flex items-center justify-center text-base-content/30 hover:text-base-content/60 hover:bg-base-200/60 transition-all duration-150 cursor-pointer shrink-0"
            >
              <ChevronRight
                className={`h-4 w-4 transition-transform duration-200 ${expanded ? 'rotate-90' : ''}`}
              />
            </button>
          </div>

          {/* Expanded details */}
          <AnimatePresence>
            {expanded && (
              <motion.div
                initial={{ height: 0, opacity: 0 }}
                animate={{ height: 'auto', opacity: 1 }}
                exit={{ height: 0, opacity: 0 }}
                transition={{ duration: 0.25, ease: [0.25, 0.1, 0.25, 1] }}
                className="overflow-hidden"
              >
                <div className="pt-4 mt-4 border-t border-base-300/30">
                  <div className="grid grid-cols-2 gap-3 mb-4">
                    {/* API Key */}
                    <div className="rounded-lg bg-base-200/40 px-3 py-2.5">
                      <div className="text-[10px] uppercase tracking-widest text-base-content/30 font-semibold mb-1.5 flex items-center gap-1.5">
                        <ShieldCheck className="h-3 w-3" />
                        API Key
                      </div>
                      <div className="font-mono text-[11px] text-base-content/50 truncate">
                        {channel.api_key ? `${channel.api_key.substring(0, 8)}••••••••${channel.api_key.substring(channel.api_key.length - 4)}` : '—'}
                      </div>
                    </div>

                    {/* Base URL */}
                    <div className="rounded-lg bg-base-200/40 px-3 py-2.5">
                    </div>
                  </div>

                  {/* Actions */}
                  <div className="flex items-center gap-2 mb-4">
                    <Link
                      to={`/console/channels/${channel.id}`}
                      className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-base-200/60 hover:bg-base-300/60 text-[11px] font-medium text-base-content/60 hover:text-base-content transition-all duration-150 cursor-pointer"
                    >
                      Configure
                    </Link>
                    <span
                      className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-base-200/40 text-[11px] font-medium text-base-content/35 cursor-default"
                    >
                      <Globe className="h-3 w-3" />
                      {providerName || channel.provider_id}
                    </span>
                  </div>

                  {/* Channel Models */}
                  {channelModels && channelModels.length > 0 && (
                    <div className="pt-3 border-t border-base-300/30">
                      <div className="text-[10px] uppercase tracking-widest text-base-content/30 font-semibold mb-2 flex items-center gap-1.5">
                        <Cpu className="h-3 w-3" />
                        Models ({channelModels.length})
                      </div>
                      <div className="flex flex-wrap gap-1.5">
                        {channelModels.map((cm) => (
                          <span
                            key={cm.id}
                            className={`inline-flex items-center gap-1 px-2 py-1 rounded-md text-[11px] font-medium ${
                              cm.enabled
                                ? 'bg-success/10 text-success/80'
                                : 'bg-base-300/30 text-base-content/40'
                            }`}
                          >
                            {cm.upstream_model_name || cm.model_id}
                          </span>
                        ))}
                      </div>
                    </div>
                  )}
                </div>
              </motion.div>
            )}
          </AnimatePresence>
        </div>
      </div>
    </motion.div>
  );
}

// ── Empty State ───────────────────────────────────────────────────────────────
function EmptyState({ onAddClick }: { onAddClick: () => void }) {
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

      <h3 className="text-[15px] font-semibold text-base-content/50 mb-1.5">No channels configured</h3>
      <p className="text-[13px] text-base-content/25 mb-8 text-center max-w-xs leading-relaxed">
        Channels connect your providers to route traffic through the gateway.
      </p>

      <button
        onClick={onAddClick}
        className="inline-flex items-center gap-2 px-5 py-2.5 rounded-xl bg-accent/10 hover:bg-accent/15 border border-accent/20 text-accent text-[13px] font-semibold transition-all duration-200 cursor-pointer"
      >
        <Plus className="h-4 w-4" />
        Add First Channel
      </button>
    </motion.div>
  );
}

// ── Stats Bar ─────────────────────────────────────────────────────────────────
function StatsBar({ channels }: { channels: Channel[] }) {
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
        <div className="text-[10px] uppercase tracking-widest text-base-content/30 font-semibold mb-1">Total Channels</div>
        <div className="text-[22px] font-bold text-base-content font-mono">{total}</div>
      </div>
      <div className="rounded-xl border border-success/20 bg-success/5 px-4 py-3">
        <div className="text-[10px] uppercase tracking-widest text-success/70 font-semibold mb-1">Active</div>
        <div className="text-[22px] font-bold text-success font-mono">{active}</div>
      </div>
      <div className="rounded-xl border border-base-300/50 bg-base-100/50 px-4 py-3">
        <div className="text-[10px] uppercase tracking-widest text-base-content/30 font-semibold mb-1">Disabled</div>
        <div className="text-[22px] font-bold text-base-content/40 font-mono">{disabled}</div>
      </div>
    </motion.div>
  );
}

// ── Main Page ─────────────────────────────────────────────────────────────────
export default function Channels() {
  const { data: channels, isLoading } = useAllChannels();
  const { data: providers } = useProviders();
  const [isAdding, setIsAdding] = useState(false);

  const getProviderName = (providerId: string) =>
    providers?.find(p => p.id === providerId)?.name ?? providerId;

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-24">
        <div className="flex flex-col items-center gap-3">
          <div className="w-8 h-8 rounded-full border-2 border-accent/30 border-t-accent animate-spin" />
          <span className="text-[12px] text-base-content/35 font-medium">Loading channels...</span>
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
            <h1 className="text-[22px] font-bold tracking-tight text-base-content">Channels</h1>
            {totalChannels > 0 && (
              <span className="inline-flex items-center px-2 py-0.5 rounded-full text-[10px] font-bold uppercase tracking-widest bg-base-200/70 text-base-content/40 border border-base-300/50">
                {totalChannels}
              </span>
            )}
          </div>
          <p className="text-[13px] text-base-content/35">
            {totalChannels === 0
              ? 'Configure provider failover endpoints'
              : `${activeChannels} active · ${totalChannels - activeChannels} disabled`}
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
                Add Channel
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
