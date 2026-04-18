import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Plus, Pencil, Trash2, ChevronRight, Copy, Check } from 'lucide-react';
import { useProviders, useCreateProvider, useUpdateProvider, useDeleteProvider } from '../hooks/useProviders';
import { Button } from '../components/ui/Button';
import { Modal } from '../components/ui/Modal';
import { Drawer } from '../components/ui/Drawer';
import { EndpointsEditor } from '../components/ui/EndpointsEditor';
import { Badge } from '../components/ui/Badge';
import type { Provider } from '../types';

const PROTOCOL_META: Record<string, { label: string; color: string; dot: string; badge: 'blue' | 'purple' }> = {
  openai:    { label: 'OpenAI',    color: 'text-info',    dot: 'bg-info',    badge: 'blue'   },
  anthropic: { label: 'Anthropic', color: 'text-primary', dot: 'bg-primary', badge: 'purple' },
};

function EndpointRow({ protocol, url }: { protocol: string; url: string }) {
  const [copied, setCopied] = useState(false);
  const meta = PROTOCOL_META[protocol] ?? { label: protocol, color: 'text-base-content/50', dot: 'bg-base-content/30', badge: 'neutral' as const };

  const handleCopy = () => {
    navigator.clipboard.writeText(url);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <div className="flex items-center gap-3 px-3.5 py-2 rounded-lg hover:bg-base-200/40 transition-colors group">
      <span className={`w-2 h-2 rounded-full shrink-0 ${meta.dot}`} />
      <span className={`text-[10px] font-semibold uppercase tracking-widest w-16 shrink-0 ${meta.color}`}>
        {meta.label}
      </span>
      <span className="font-mono text-[12px] text-base-content/50 truncate flex-1 min-w-0">
        {url}
      </span>
      <button
        onClick={handleCopy}
        className="opacity-0 group-hover:opacity-100 transition-opacity p-1 rounded hover:bg-base-300/50"
        aria-label="Copy endpoint URL"
      >
        {copied
          ? <Check className="h-3 w-3 text-success" />
          : <Copy className="h-3 w-3 text-base-content/30" />
        }
      </button>
    </div>
  );
}

function ProviderCard({ provider, onEdit, onDelete, navigate }: {
  provider: Provider;
  onEdit: (p: Provider) => void;
  onDelete: (p: Provider) => void;
  navigate: ReturnType<typeof useNavigate>;
}) {
  const endpointEntries = provider.endpoints ? Object.entries(provider.endpoints) : [];
  const hasEndpoints = endpointEntries.length > 0;

  return (
    <div className={`
      rounded-2xl border transition-all duration-200 overflow-hidden
      ${provider.enabled
        ? 'bg-base-100 border-base-300/50 hover:border-primary/20 hover:shadow-[0_0_16px_-4px_rgba(var(--primary),0.08)]'
        : 'bg-base-100/40 border-base-300/25'
      }
    `}>
      {/* Top accent line */}
      <div className={`h-[2px] w-full ${provider.enabled ? 'bg-gradient-to-r from-primary/60 via-primary/20 to-transparent' : 'bg-gradient-to-r from-base-300/30 to-transparent'}`} />

      <div className="p-5">
        {/* Header row */}
        <div className="flex items-start justify-between mb-4">
          <div className="flex items-center gap-3 min-w-0">
            <div className={`
              w-9 h-9 rounded-xl flex items-center justify-center shrink-0
              ${provider.enabled ? 'bg-primary/10 ring-1 ring-primary/20' : 'bg-base-200/60'}
            `}>
              <span className={`text-[11px] font-black tracking-tight ${provider.enabled ? 'text-primary' : 'text-base-content/30'}`}>
                {provider.name.slice(0, 2).toUpperCase()}
              </span>
            </div>
            <div className="min-w-0">
              <div className={`font-mono text-[15px] font-bold leading-tight truncate ${provider.enabled ? 'text-base-content' : 'text-base-content/40'}`}>
                {provider.name}
              </div>
              <div className="flex items-center gap-1.5 mt-0.5">
                <span className={`w-1.5 h-1.5 rounded-full ${provider.enabled ? 'bg-success' : 'bg-base-content/20'}`} />
                <span className={`text-[11px] ${provider.enabled ? 'text-base-content/50' : 'text-base-content/20'}`}>
                  {provider.enabled ? 'Active' : 'Disabled'}
                </span>
              </div>
            </div>
          </div>

          <div className="flex items-center gap-1 shrink-0">
            <button onClick={() => onEdit(provider)} className="btn btn-ghost btn-xs btn-circle" aria-label="Edit provider">
              <Pencil className="h-3.5 w-3.5" />
            </button>
            <button onClick={() => onDelete(provider)} className="btn btn-ghost btn-xs btn-circle text-error/60 hover:text-error" aria-label="Delete provider">
              <Trash2 className="h-3.5 w-3.5" />
            </button>
          </div>
        </div>

        {/* Protocol badges */}
        <div className="flex flex-wrap gap-1.5 mb-4">
          {endpointEntries.map(([protocol]) => {
            const meta = PROTOCOL_META[protocol];
            if (!meta) return <Badge key={protocol} variant="neutral">{protocol}</Badge>;
            return <Badge key={protocol} variant={meta.badge}>{meta.label}</Badge>;
          })}
          {!hasEndpoints && (
            <span className="text-[11px] text-base-content/25 italic">No endpoints</span>
          )}
        </div>

        {/* Endpoint details */}
        {hasEndpoints && (
          <div className="rounded-xl border border-base-200/60 bg-base-200/20 overflow-hidden divide-y divide-base-200/40 mb-3">
            {endpointEntries.map(([protocol, url]) => (
              <EndpointRow key={protocol} protocol={protocol} url={url} />
            ))}
          </div>
        )}

        {/* Footer */}
        <div className="flex items-center justify-between pt-2 border-t border-base-200/40">
          <span className="font-mono text-[11px] text-base-content/20">
            {new Date(provider.created_at).toLocaleDateString()}
          </span>
          <button
            onClick={() => navigate(`/console/providers/${provider.id}`)}
            className="text-[11px] font-medium text-primary/60 hover:text-primary transition-colors flex items-center gap-0.5"
          >
            Channels <ChevronRight className="h-3 w-3" />
          </button>
        </div>
      </div>
    </div>
  );
}

export default function Providers() {
  const { data: providers, isLoading } = useProviders();
  const createMutation = useCreateProvider();
  const updateMutation = useUpdateProvider();
  const deleteMutation = useDeleteProvider();
  const navigate = useNavigate();

  // Create modal state
  const [createOpen, setCreateOpen] = useState(false);
  const [name, setName] = useState('');
  const [createEndpoints, setCreateEndpoints] = useState<Record<string, string>>({});

  // Edit modal state
  const [editOpen, setEditOpen] = useState(false);
  const [editingProvider, setEditingProvider] = useState<Provider | null>(null);
  const [editName, setEditName] = useState('');
  const [editEndpoints, setEditEndpoints] = useState<Record<string, string>>({});
  const [editEnabled, setEditEnabled] = useState(true);

  // Delete modal state
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
      endpoints: Object.keys(endpoints).length > 0 ? JSON.stringify(endpoints) : null,
    });
    setName('');
    setCreateEndpoints({});
    setCreateOpen(false);
  };

  const handleEdit = (provider: Provider) => {
    // endpoints is now already parsed as object
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
        endpoints: Object.keys(endpoints).length > 0 ? JSON.stringify(endpoints) : null,
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

  return (
    <div>
      <div className="mb-8 flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Providers</h1>
          <p className="text-sm text-base-content/40 mt-1">Configure upstream LLM provider endpoints</p>
        </div>
        <Button icon={<Plus className="h-4 w-4" />} onClick={() => setCreateOpen(true)}>
          Add Provider
        </Button>
      </div>

      {isLoading ? (
        <div className="flex items-center justify-center py-20"><span className="loading loading-spinner loading-lg" /></div>
      ) : providers?.length ? (
        <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
          {providers.map((provider) => (
            <ProviderCard
              key={provider.id}
              provider={provider}
              onEdit={handleEdit}
              onDelete={handleDelete}
              navigate={navigate}
            />
          ))}
        </div>
      ) : (
        <div className="flex flex-col items-center justify-center py-24 rounded-2xl border border-dashed border-base-300/40">
          <div className="w-12 h-12 rounded-2xl bg-base-200/60 flex items-center justify-center mb-4">
            <span className="text-2xl opacity-30">⊞</span>
          </div>
          <p className="text-base-content/30 text-sm mb-4">No providers configured</p>
          <Button icon={<Plus className="h-4 w-4" />} onClick={() => setCreateOpen(true)}>
            Add your first provider
          </Button>
        </div>
      )}

      <Drawer open={createOpen} onClose={() => setCreateOpen(false)} title="Add Provider">
        <form onSubmit={handleCreate} className="space-y-4">
          <div className="form-control">
            <label className="label">
              <span className="label-text font-medium">Name</span>
            </label>
            <input type="text" value={name} onChange={(e) => setName(e.target.value)} placeholder="e.g., OpenAI" required className="input input-bordered w-full" />
          </div>
          <div className="form-control">
            <label className="label">
              <span className="label-text font-medium">Endpoints</span>
            </label>
            <EndpointsEditor
              value={createEndpoints}
              onChange={setCreateEndpoints}
            />
          </div>
          <div className="flex justify-end pt-4">
            <Button variant="primary" loading={createMutation.isPending}>Create</Button>
          </div>
        </form>
      </Drawer>

      {/* Edit Drawer */}
      <Drawer open={editOpen} onClose={() => setEditOpen(false)} title="Edit Provider">
        <form onSubmit={handleUpdate} className="space-y-4">
          <div className="form-control">
            <label className="label">
              <span className="label-text font-medium">Name</span>
            </label>
            <input type="text" value={editName} onChange={(e) => setEditName(e.target.value)} placeholder="e.g., OpenAI" required className="input input-bordered w-full" />
          </div>
          <div className="form-control">
            <label className="label">
              <span className="label-text font-medium">Endpoints</span>
            </label>
            <EndpointsEditor
              value={editEndpoints}
              onChange={setEditEndpoints}
            />
          </div>
          <div className="form-control">
            <label className="label cursor-pointer justify-start gap-3">
              <input
                type="checkbox"
                checked={editEnabled}
                onChange={(e) => setEditEnabled(e.target.checked)}
                className="checkbox checkbox-primary"
              />
              <span className="label-text font-medium">Enabled</span>
            </label>
          </div>
          <div className="flex justify-end gap-2 pt-4">
            <Button variant="primary" loading={updateMutation.isPending}>Save Changes</Button>
          </div>
        </form>
      </Drawer>

      {/* Delete Confirmation Modal */}
      <Modal open={deleteOpen} onClose={() => setDeleteOpen(false)} title="Delete Provider">
        <div className="space-y-4">
          <p className="text-base-content/70">
            Are you sure you want to delete <strong>{deletingProvider?.name}</strong>? This will also remove all associated channels.
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