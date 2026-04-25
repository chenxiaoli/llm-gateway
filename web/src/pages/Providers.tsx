import { useState } from 'react';
import { Plus, Pencil, Trash2, Copy, Check, Globe, Zap, Radio, Layers } from 'lucide-react';
import { useProviders, useCreateProvider, useUpdateProvider, useDeleteProvider } from '../hooks/useProviders';
import { Button } from '../components/ui/Button';
import { Modal } from '../components/ui/Modal';
import { Drawer } from '../components/ui/Drawer';
import { EndpointsEditor } from '../components/ui/EndpointsEditor';
import { motion, AnimatePresence } from 'framer-motion';
import type { Provider } from '../types';

// ── Protocol identity system ────────────────────────────────────────────────
const PROTOCOL_THEME: Record<string, {
  label: string;
  accent: string;
  glow: string;
  bg: string;
  border: string;
  text: string;
}> = {
  openai: {
    label: 'OpenAI',
    accent: '#06d6a0',
    glow: 'rgba(6, 214, 160, 0.15)',
    bg: 'rgba(6, 214, 160, 0.06)',
    border: 'rgba(6, 214, 160, 0.20)',
    text: 'rgba(6, 214, 160, 0.85)',
  },
  anthropic: {
    label: 'Anthropic',
    accent: '#f59e0b',
    glow: 'rgba(245, 158, 11, 0.15)',
    bg: 'rgba(245, 158, 11, 0.06)',
    border: 'rgba(245, 158, 11, 0.20)',
    text: 'rgba(245, 158, 11, 0.85)',
  },
};

function getProtocolTheme(protocol: string) {
  return PROTOCOL_THEME[protocol] ?? {
    label: protocol,
    accent: '#6b7280',
    glow: 'rgba(107, 114, 128, 0.15)',
    bg: 'rgba(107, 114, 128, 0.06)',
    border: 'rgba(107, 114, 128, 0.20)',
    text: 'rgba(107, 114, 128, 0.85)',
  };
}

// ── Endpoint readout ────────────────────────────────────────────────────────
function EndpointReadout({ protocol, url }: { protocol: string; url: string }) {
  const [copied, setCopied] = useState(false);
  const theme = getProtocolTheme(protocol);

  const handleCopy = () => {
    navigator.clipboard.writeText(url);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <div
      className="flex items-center gap-3 px-3.5 py-2.5 group cursor-pointer transition-all duration-200"
      style={{ background: theme.bg }}
      onClick={handleCopy}
    >
      {/* Protocol dot + label */}
      <div className="flex items-center gap-2 shrink-0 w-24">
        <span
          className="w-1.5 h-1.5 rounded-full shrink-0"
          style={{ background: theme.accent, boxShadow: `0 0 6px ${theme.accent}` }}
        />
        <span
          className="text-xs font-mono font-bold uppercase tracking-[0.12em]"
          style={{ color: theme.text }}
        >
          {theme.label}
        </span>
      </div>

      {/* URL */}
      <code className="font-mono text-base text-base-content/60 truncate flex-1 min-w-0">
        {url}
      </code>

      {/* Copy action */}
      <div className="opacity-0 group-hover:opacity-100 transition-opacity shrink-0">
        {copied
          ? <Check className="h-3.5 w-3.5 text-success" />
          : <Copy className="h-3.5 w-3.5 text-base-content/40" />
        }
      </div>
    </div>
  );
}

// ── Provider Module ─────────────────────────────────────────────────────────
function ProviderModule({ provider, onEdit, onDelete, index }: {
  provider: Provider;
  onEdit: (p: Provider) => void;
  onDelete: (p: Provider) => void;
  index: number;
}) {
  const endpointEntries = provider.endpoints ? Object.entries(provider.endpoints) : [];
  const hasEndpoints = endpointEntries.length > 0;

  // Derive the primary protocol for the card's accent color
  const primaryProtocol = endpointEntries[0]?.[0];
  const primaryTheme = primaryProtocol ? getProtocolTheme(primaryProtocol) : null;

  return (
    <motion.div
      initial={{ opacity: 0, y: 20, scale: 0.97 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      transition={{ duration: 0.45, delay: index * 0.08, ease: [0.25, 0.1, 0.25, 1] }}
      className="group relative"
    >
      <div
        className={`
          relative overflow-hidden rounded-xl border transition-all duration-300
          ${provider.enabled
            ? 'bg-base-100 border-base-300/60 hover:border-base-content/15'
            : 'bg-base-100/30 border-base-300/20 opacity-60 hover:opacity-80'
          }
        `}
      >
        {/* Top accent bar — protocol-colored */}
        <div
          className="h-[2px] w-full"
          style={{
            background: provider.enabled && primaryTheme
              ? `linear-gradient(to right, ${primaryTheme.accent}60, ${primaryTheme.accent}15, transparent)`
              : 'linear-gradient(to right, rgba(255,255,255,0.06), transparent)',
          }}
        />

        <div className="p-5">
          {/* Header */}
          <div className="flex items-start justify-between mb-5">
            <div className="flex items-center gap-3.5 min-w-0">
              {/* Monogram */}
              <div
                className="w-10 h-10 rounded-lg flex items-center justify-center shrink-0 font-mono font-black text-base tracking-tight"
                style={{
                  background: provider.enabled && primaryTheme ? primaryTheme.bg : 'rgba(255,255,255,0.04)',
                  color: provider.enabled && primaryTheme ? primaryTheme.text : 'rgba(255,255,255,0.35)',
                  border: provider.enabled && primaryTheme ? `1px solid ${primaryTheme.border}` : '1px solid rgba(255,255,255,0.06)',
                }}
              >
                {provider.name.slice(0, 2).toUpperCase()}
              </div>
              <div className="min-w-0">
                <h3 className="font-mono text-lg font-bold text-base-content/90 truncate leading-tight">
                  {provider.name}
                </h3>
                <div className="flex items-center gap-2 mt-1">
                  {/* Status indicator */}
                  <span className="flex items-center gap-1.5">
                    <span
                      className={`w-1.5 h-1.5 rounded-full ${provider.enabled ? '' : ''}`}
                      style={provider.enabled ? {
                        background: '#06d6a0',
                        boxShadow: '0 0 8px rgba(6, 214, 160, 0.5)',
                      } : { background: 'rgba(255,255,255,0.15)' }}
                    />
                    <span className={`text-base font-medium ${provider.enabled ? 'text-base-content/60' : 'text-base-content/35'}`}>
                      {provider.enabled ? 'Active' : 'Disabled'}
                    </span>
                  </span>

                  {/* Protocol tags */}
                  {endpointEntries.map(([protocol]) => {
                    const theme = getProtocolTheme(protocol);
                    return (
                      <span
                        key={protocol}
                        className="text-xs font-mono font-semibold uppercase tracking-[0.1em] px-1.5 py-0.5 rounded"
                        style={{
                          color: theme.text,
                          background: theme.bg,
                          border: `1px solid ${theme.border}`,
                        }}
                      >
                        {theme.label}
                      </span>
                    );
                  })}
                </div>
              </div>
            </div>

            {/* Actions */}
            <div className="flex items-center gap-0.5 shrink-0 opacity-0 group-hover:opacity-100 transition-opacity duration-200">
              <button
                onClick={() => onEdit(provider)}
                className="w-8 h-8 rounded-lg flex items-center justify-center text-base-content/45 hover:text-base-content/70 hover:bg-base-200/60 transition-all duration-150 cursor-pointer"
                aria-label="Edit provider"
              >
                <Pencil className="h-3.5 w-3.5" />
              </button>
              <button
                onClick={() => onDelete(provider)}
                className="w-8 h-8 rounded-lg flex items-center justify-center text-base-content/45 hover:text-danger hover:bg-danger/10 transition-all duration-150 cursor-pointer"
                aria-label="Delete provider"
              >
                <Trash2 className="h-3.5 w-3.5" />
              </button>
            </div>
          </div>

          {/* Endpoint readouts */}
          {hasEndpoints ? (
            <div className="rounded-lg overflow-hidden border border-base-300/30 divide-y divide-base-300/20">
              {endpointEntries.map(([protocol, url]) => (
                <EndpointReadout key={protocol} protocol={protocol} url={url} />
              ))}
            </div>
          ) : (
            <div className="rounded-lg border border-dashed border-base-300/25 py-4 flex items-center justify-center">
              <span className="text-base text-base-content/35 italic">No endpoints configured</span>
            </div>
          )}

          {/* Footer — metadata */}
          <div className="flex items-center justify-between mt-4 pt-3 border-t border-base-300/20">
            <span className="font-mono text-xs text-base-content/35">
              {new Date(provider.created_at).toLocaleDateString()}
            </span>
            {provider.enabled && hasEndpoints && (
              <span className="flex items-center gap-1.5 text-xs text-base-content/35">
                <Radio className="h-3 w-3" />
                {endpointEntries.length} endpoint{endpointEntries.length > 1 ? 's' : ''}
              </span>
            )}
          </div>
        </div>
      </div>
    </motion.div>
  );
}

// ── Empty State ─────────────────────────────────────────────────────────────
function EmptyState({ onAddClick }: { onAddClick: () => void }) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 24 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.5 }}
      className="flex flex-col items-center justify-center py-28 px-4"
    >
      <div className="relative mb-8">
        <div className="w-20 h-20 rounded-2xl bg-base-100 border border-base-300/30 flex items-center justify-center">
          <Layers className="h-9 w-9 text-base-content/20" />
        </div>
        <div className="absolute inset-0 rounded-2xl border border-dashed border-base-300/20 -m-4" />
        <div className="absolute -top-2 -right-2 w-7 h-7 rounded-lg bg-accent/10 border border-accent/20 flex items-center justify-center">
          <Plus className="h-4 w-4 text-accent" />
        </div>
      </div>

      <h3 className="text-lg font-semibold text-base-content/60 mb-2">No providers configured</h3>
      <p className="text-base text-base-content/40 mb-8 text-center max-w-sm leading-relaxed">
        Connect upstream LLM providers to route traffic through the gateway.
      </p>

      <button
        onClick={onAddClick}
        className="inline-flex items-center gap-2 px-6 py-3 rounded-xl bg-accent/10 hover:bg-accent/15 border border-accent/25 text-accent text-base font-semibold transition-all duration-200 cursor-pointer"
      >
        <Plus className="h-4.5 w-4.5" />
        Add First Provider
      </button>
    </motion.div>
  );
}

// ── Stats ───────────────────────────────────────────────────────────────────
function StatsBar({ providers }: { providers: Provider[] }) {
  const total = providers.length;
  const active = providers.filter(p => p.enabled).length;
  const endpointCount = providers.reduce((acc, p) => {
    return acc + (p.endpoints ? Object.keys(p.endpoints).length : 0);
  }, 0);

  return (
    <motion.div
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.4, delay: 0.05 }}
      className="grid grid-cols-3 gap-3 mb-8"
    >
      {[
        { label: 'Providers', value: total, color: 'text-base-content/70', border: 'border-base-300/40' },
        { label: 'Active', value: active, color: 'text-success', border: 'border-success/15' },
        { label: 'Endpoints', value: endpointCount, color: 'text-base-content/60', border: 'border-base-300/30' },
      ].map((stat) => (
        <div key={stat.label} className={`rounded-xl border bg-base-100/40 px-4 py-3 ${stat.border}`}>
          <div className="text-xs uppercase tracking-[0.12em] text-base-content/45 font-semibold mb-1">
            {stat.label}
          </div>
          <div className={`text-xl font-bold font-mono ${stat.color}`}>
            {stat.value}
          </div>
        </div>
      ))}
    </motion.div>
  );
}

// ── Main Page ───────────────────────────────────────────────────────────────
export default function Providers() {
  const { data: providers, isLoading } = useProviders();
  const createMutation = useCreateProvider();
  const updateMutation = useUpdateProvider();
  const deleteMutation = useDeleteProvider();

  const [createOpen, setCreateOpen] = useState(false);
  const [name, setName] = useState('');
  const [createEndpoints, setCreateEndpoints] = useState<Record<string, string>>({});

  const [editOpen, setEditOpen] = useState(false);
  const [editingProvider, setEditingProvider] = useState<Provider | null>(null);
  const [editName, setEditName] = useState('');
  const [editEndpoints, setEditEndpoints] = useState<Record<string, string>>({});
  const [editEnabled, setEditEnabled] = useState(true);

  const [deleteOpen, setDeleteOpen] = useState(false);
  const [deletingProvider, setDeletingProvider] = useState<Provider | null>(null);

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    const endpoints: Record<string, string | null> = {};
    Object.entries(createEndpoints).forEach(([key, value]) => {
      endpoints[key] = value || null;
    });
    await createMutation.mutateAsync({
      name,
      endpoints: Object.keys(endpoints).length > 0 ? endpoints : null,
    });
    setName('');
    setCreateEndpoints({});
    setCreateOpen(false);
  };

  const handleEdit = (provider: Provider) => {
    setEditingProvider(provider);
    setEditName(provider.name);
    setEditEndpoints(provider.endpoints || {});
    setEditEnabled(provider.enabled);
    setEditOpen(true);
  };

  const handleUpdate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!editingProvider) return;
    const endpoints: Record<string, string | null> = {};
    Object.entries(editEndpoints).forEach(([key, value]) => {
      endpoints[key] = value || null;
    });
    await updateMutation.mutateAsync({
      id: editingProvider.id,
      input: {
        name: editName,
        endpoints: Object.keys(endpoints).length > 0 ? endpoints : null,
        enabled: editEnabled,
      },
    });
    setEditOpen(false);
    setEditingProvider(null);
  };

  const handleDelete = (provider: Provider) => {
    setDeletingProvider(provider);
    setDeleteOpen(true);
  };

  const confirmDelete = async () => {
    if (!deletingProvider) return;
    await deleteMutation.mutateAsync(deletingProvider.id);
    setDeleteOpen(false);
    setDeletingProvider(null);
  };

  const totalProviders = providers?.length ?? 0;

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
            <h1 className="text-xl font-bold tracking-tight text-base-content">Providers</h1>
            {totalProviders > 0 && (
              <span className="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-bold uppercase tracking-[0.12em] bg-base-200/70 text-base-content/45 border border-base-300/40">
                {totalProviders}
              </span>
            )}
          </div>
          <p className="text-base text-base-content/50">
            Upstream LLM provider endpoints and routing configuration
          </p>
        </div>

        <AnimatePresence>
          {totalProviders > 0 && (
            <motion.div
              initial={{ opacity: 0, scale: 0.92 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={{ opacity: 0, scale: 0.92 }}
              transition={{ duration: 0.2 }}
            >
              <Button
                icon={<Plus className="h-4 w-4" />}
                size="sm"
                onClick={() => setCreateOpen(true)}
              >
                Add Provider
              </Button>
            </motion.div>
          )}
        </AnimatePresence>
      </motion.div>

      {/* Stats */}
      {totalProviders > 0 && <StatsBar providers={providers!} />}

      {/* Content */}
      {isLoading ? (
        <div className="flex items-center justify-center py-24">
          <div className="flex flex-col items-center gap-3">
            <div className="w-8 h-8 rounded-full border-2 border-accent/30 border-t-accent animate-spin" />
            <span className="text-base text-base-content/45 font-medium">Loading providers...</span>
          </div>
        </div>
      ) : totalProviders === 0 ? (
        <EmptyState onAddClick={() => setCreateOpen(true)} />
      ) : (
        <motion.div
          initial="hidden"
          animate="visible"
          variants={{ hidden: {}, visible: { transition: { staggerChildren: 0.08 } } }}
          className="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-4"
        >
          {providers!.map((provider, i) => (
            <ProviderModule
              key={provider.id}
              provider={provider}
              onEdit={handleEdit}
              onDelete={handleDelete}
              index={i}
            />
          ))}
        </motion.div>
      )}

      {/* Create Drawer */}
      <Drawer open={createOpen} onClose={() => setCreateOpen(false)} title="Add Provider" width={440}>
        <form onSubmit={handleCreate} className="space-y-6">
          <div className="space-y-1.5">
            <label className="text-xs font-semibold uppercase tracking-wider text-base-content/55 flex items-center gap-1.5">
              <Globe className="h-3.5 w-3.5" />
              Name
            </label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="e.g., OpenAI"
              required
              className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-base font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
            />
          </div>
          <div className="space-y-1.5">
            <label className="text-xs font-semibold uppercase tracking-wider text-base-content/55 flex items-center gap-1.5">
              <Zap className="h-3.5 w-3.5" />
              Endpoints
            </label>
            <EndpointsEditor value={createEndpoints} onChange={setCreateEndpoints} />
          </div>
          <div className="flex items-center gap-2 pt-2">
            <Button type="submit" variant="primary" loading={createMutation.isPending} className="flex-1">
              Create Provider
            </Button>
            <Button type="button" variant="ghost" onClick={() => setCreateOpen(false)}>
              Cancel
            </Button>
          </div>
        </form>
      </Drawer>

      {/* Edit Drawer */}
      <Drawer open={editOpen} onClose={() => setEditOpen(false)} title="Edit Provider" width={440}>
        <form onSubmit={handleUpdate} className="space-y-6">
          <div className="space-y-1.5">
            <label className="text-xs font-semibold uppercase tracking-wider text-base-content/55 flex items-center gap-1.5">
              <Globe className="h-3.5 w-3.5" />
              Name
            </label>
            <input
              type="text"
              value={editName}
              onChange={(e) => setEditName(e.target.value)}
              placeholder="e.g., OpenAI"
              required
              className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-base font-mono text-base-content placeholder:text-base-content/20 focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
            />
          </div>
          <div className="space-y-1.5">
            <label className="text-xs font-semibold uppercase tracking-wider text-base-content/55 flex items-center gap-1.5">
              <Zap className="h-3.5 w-3.5" />
              Endpoints
            </label>
            <EndpointsEditor value={editEndpoints} onChange={setEditEndpoints} />
          </div>
          <div className="flex items-center justify-between">
            <div className="space-y-0.5">
              <span className="text-base font-medium text-base-content">Enabled</span>
              <p className="text-xs text-base-content/50">Provider must be enabled to receive traffic</p>
            </div>
            <input
              type="checkbox"
              checked={editEnabled}
              onChange={(e) => setEditEnabled(e.target.checked)}
              className="checkbox checkbox-primary"
            />
          </div>
          <div className="flex items-center gap-2 pt-2">
            <Button type="submit" variant="primary" loading={updateMutation.isPending} className="flex-1">
              Save Changes
            </Button>
            <Button type="button" variant="ghost" onClick={() => setEditOpen(false)}>
              Cancel
            </Button>
          </div>
        </form>
      </Drawer>

      {/* Delete Confirmation */}
      <Modal open={deleteOpen} onClose={() => setDeleteOpen(false)} title="Delete Provider">
        <div className="space-y-4">
          <p className="text-base-content/70">
            Are you sure you want to delete <strong className="text-base-content">{deletingProvider?.name}</strong>?
            This will also remove all associated channels.
          </p>
          <div className="flex justify-end gap-2">
            <Button onClick={() => setDeleteOpen(false)}>Cancel</Button>
            <Button variant="danger" loading={deleteMutation.isPending} onClick={confirmDelete}>Delete</Button>
          </div>
        </div>
      </Modal>
    </div>
  );
}
