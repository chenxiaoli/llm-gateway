import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Plus, Pencil, Trash2 } from 'lucide-react';
import { useProviders, useCreateProvider, useUpdateProvider, useDeleteProvider } from '../hooks/useProviders';
import { Button } from '../components/ui/Button';
import { Modal } from '../components/ui/Modal';
import { Drawer } from '../components/ui/Drawer';
import { EndpointsEditor } from '../components/ui/EndpointsEditor';
import { Badge } from '../components/ui/Badge';
import type { Provider } from '../types';

export default function Providers() {
  const { data: providers, isLoading } = useProviders();
  const createMutation = useCreateProvider();
  const updateMutation = useUpdateProvider();
  const deleteMutation = useDeleteProvider();
  const navigate = useNavigate();

  // Create modal state
  const [createOpen, setCreateOpen] = useState(false);
  const [name, setName] = useState('');
  const [baseUrl, setBaseUrl] = useState('');
  const [createEndpoints, setCreateEndpoints] = useState<Record<string, string>>({});

  // Edit modal state
  const [editOpen, setEditOpen] = useState(false);
  const [editingProvider, setEditingProvider] = useState<Provider | null>(null);
  const [editName, setEditName] = useState('');
  const [editBaseUrl, setEditBaseUrl] = useState('');
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
      base_url: baseUrl || null,
      endpoints: Object.keys(endpoints).length > 0 ? JSON.stringify(endpoints) : null,
    });
    setName('');
    setBaseUrl('');
    setCreateEndpoints({});
    setCreateOpen(false);
  };

  const handleEdit = (provider: Provider) => {
    let parsedEndpoints: Record<string, string> = {};
    if (provider.endpoints) {
      try {
        parsedEndpoints = JSON.parse(provider.endpoints);
      } catch {}
    }
    setEditingProvider(provider);
    setEditName(provider.name);
    setEditBaseUrl(provider.base_url || '');
    setEditEndpoints(parsedEndpoints);
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
        base_url: editBaseUrl || null,
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
        <div className="flex items-center justify-center py-12"><span className="loading loading-spinner loading-lg" /></div>
      ) : (
        <div className="rounded-xl border border-base-300/50 bg-base-100/60 overflow-hidden">
          <div className="overflow-x-auto">
            <table className="table table-sm">
              <thead>
                <tr className="border-b border-base-300/50">
                  <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/35">Name</th>
                  <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/35">Protocols</th>
                  <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/35">Status</th>
                  <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/35">Created</th>
                  <th className="text-[11px] font-semibold uppercase tracking-wider text-base-content/35 w-20">Actions</th>
                </tr>
              </thead>
              <tbody>
                {providers?.map((provider) => (
                  <tr key={provider.id} className="border-b border-base-200/50 hover:bg-base-200/30 transition-colors">
                    <td>
                      <button
                        onClick={() => navigate(`/console/providers/${provider.id}`)}
                        className="link link-primary text-sm font-medium focus:outline-none focus:rounded-md focus:ring-2 focus:ring-primary/50 focus:ring-offset-1 focus:ring-offset-base-100"
                      >
                        {provider.name}
                      </button>
                    </td>
                    <td>
                      <div className="flex gap-1.5">
                        {(() => {
                          let hasOpenai = false;
                          let hasAnthropic = false;
                          if (provider.endpoints) {
                            try {
                              const parsed = JSON.parse(provider.endpoints);
                              hasOpenai = !!parsed.openai;
                              hasAnthropic = !!parsed.anthropic;
                            } catch {}
                          }
                          return (
                            <>
                              {hasOpenai && <Badge variant="blue">OpenAI</Badge>}
                              {hasAnthropic && <Badge variant="purple">Anthropic</Badge>}
                            </>
                          );
                        })()}
                      </div>
                    </td>
                    <td><Badge variant={provider.enabled ? 'green' : 'red'}>{provider.enabled ? 'Active' : 'Disabled'}</Badge></td>
                    <td className="mono text-[13px] text-base-content/50">{new Date(provider.created_at).toLocaleDateString()}</td>
                    <td>
                      <div className="flex items-center gap-1">
                        <button
                          onClick={() => handleEdit(provider)}
                          className="btn btn-ghost btn-xs btn-circle"
                          aria-label="Edit provider"
                        >
                          <Pencil className="h-3.5 w-3.5" />
                        </button>
                        <button
                          onClick={() => handleDelete(provider)}
                          className="btn btn-ghost btn-xs btn-circle text-error hover:text-error"
                          aria-label="Delete provider"
                        >
                          <Trash2 className="h-3.5 w-3.5" />
                        </button>
                      </div>
                    </td>
                  </tr>
                ))}
                {(!providers?.length) && (
                  <tr>
                    <td colSpan={5} className="text-center py-16">
                      <div className="flex flex-col items-center gap-2">
                        <span className="text-base-content/25 text-sm">No providers configured</span>
                        <button onClick={() => setCreateOpen(true)} className="link link-primary text-sm">
                          Add your first provider
                        </button>
                      </div>
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
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
              <span className="label-text font-medium">Base URL (Fallback)</span>
              <span className="label-text-alt">Optional</span>
            </label>
            <input type="text" value={baseUrl} onChange={(e) => setBaseUrl(e.target.value)} placeholder="https://api.openai.com/v1" className="input input-bordered w-full" />
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
              <span className="label-text font-medium">Base URL (Fallback)</span>
              <span className="label-text-alt">Optional</span>
            </label>
            <input type="text" value={editBaseUrl} onChange={(e) => setEditBaseUrl(e.target.value)} placeholder="https://api.openai.com/v1" className="input input-bordered w-full" />
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