import { useState } from 'react';
import { Plus, Trash2, X } from 'lucide-react';
import { useModelFallbacks, useCreateModelFallback, useUpdateModelFallback, useDeleteModelFallback } from '../hooks/useModelFallbacks';
import { Button } from '../components/ui/Button';
import { Modal } from '../components/ui/Modal';
import type { ModelFallbackGroup } from '../types';

export default function ModelFallbacks() {
  const { data: fallbacks, isLoading } = useModelFallbacks();
  const createMutation = useCreateModelFallback();
  const updateMutation = useUpdateModelFallback();
  const deleteMutation = useDeleteModelFallback();

  const [createOpen, setCreateOpen] = useState(false);
  const [editId, setEditId] = useState<string | null>(null);
  const [name, setName] = useState('');
  const [groups, setGroups] = useState<ModelFallbackGroup[]>([{ models: [''], priorities: [1] }]);
  const [deleteId, setDeleteId] = useState<string | null>(null);

  const resetForm = () => { setName(''); setGroups([{ models: [''], priorities: [1] }]); setEditId(null); };
  const openCreate = () => { resetForm(); setCreateOpen(true); };
  const openEdit = (id: string) => {
    const fb = fallbacks?.find((f) => f.id === id);
    if (!fb) return;
    setEditId(id); setName(fb.name);
    setGroups(fb.config.length > 0 ? fb.config : [{ models: [''], priorities: [1] }]);
    setCreateOpen(true);
  };

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    const cleanedGroups = groups.map((g) => ({
      models: g.models.filter((m) => m.trim() !== ''),
      priorities: g.priorities,
    })).filter((g) => g.models.length > 1);
    if (cleanedGroups.length === 0) return;
    if (editId) {
      await updateMutation.mutateAsync({ id: editId, input: { name, config: cleanedGroups } });
    } else {
      await createMutation.mutateAsync({ name, config: cleanedGroups });
    }
    setCreateOpen(false); resetForm();
  };

  const addGroup = () => setGroups([...groups, { models: [''], priorities: [1] }]);
  const removeGroup = (gi: number) => setGroups(groups.filter((_, i) => i !== gi));
  const addModel = (gi: number) => {
    const updated = [...groups];
    const group = { ...updated[gi] };
    group.models = [...group.models, ''];
    group.priorities = [...group.priorities, group.priorities.length + 1];
    updated[gi] = group;
    setGroups(updated);
  };
  const removeModel = (gi: number, mi: number) => {
    const updated = [...groups];
    const group = { ...updated[gi] };
    group.models = group.models.filter((_, i) => i !== mi);
    group.priorities = group.priorities.filter((_, i) => i !== mi);
    if (group.models.length === 0) { group.models = ['']; group.priorities = [1]; }
    updated[gi] = group;
    setGroups(updated);
  };
  const updateModelName = (gi: number, mi: number, value: string) => {
    const updated = [...groups]; const group = { ...updated[gi] }; group.models = [...group.models]; group.models[mi] = value; updated[gi] = group; setGroups(updated);
  };
  const updatePriority = (gi: number, mi: number, value: number) => {
    const updated = [...groups]; const group = { ...updated[gi] }; group.priorities = [...group.priorities]; group.priorities[mi] = value; updated[gi] = group; setGroups(updated);
  };
  const handleDelete = async () => { if (!deleteId) return; await deleteMutation.mutateAsync(deleteId); setDeleteId(null); };

  return (
    <div>
      <div className="mb-8 flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Model Fallbacks</h1>
          <p className="text-sm text-base-content/40 mt-1">Configure fallback model chains for API keys</p>
        </div>
        <Button icon={<Plus className="h-4 w-4" />} onClick={openCreate}>Create Fallback</Button>
      </div>

      {isLoading ? (
        <div className="flex items-center justify-center py-12"><span className="loading loading-spinner loading-lg" /></div>
      ) : (
        <div className="space-y-4">
          {fallbacks?.map((fb) => (
            <div key={fb.id} className="rounded-xl border border-base-300/50 bg-base-100/60 p-5">
              <div className="flex items-center justify-between mb-3">
                <div>
                  <h3 className="font-semibold text-sm">{fb.name}</h3>
                  <p className="text-xs text-base-content/40 mt-0.5">{fb.config.length} group(s)</p>
                </div>
                <div className="flex gap-2">
                  <Button variant="ghost" size="sm" onClick={() => openEdit(fb.id)}>Edit</Button>
                  <Button variant="ghost" size="sm" onClick={() => setDeleteId(fb.id)}><Trash2 className="h-4 w-4 text-error" /></Button>
                </div>
              </div>
              <div className="space-y-2">
                {fb.config.map((group, gi) => (
                  <div key={gi} className="rounded-lg border border-base-200/50 bg-base-200/30 p-3">
                    <div className="flex flex-wrap gap-2">
                      {group.models.map((model, mi) => (
                        <span key={mi} className="inline-flex items-center gap-1 rounded-md bg-base-200 px-2 py-1 text-xs font-mono">
                          <span className="text-base-content/30">P{group.priorities[mi]}</span>
                          <span>{model}</span>
                        </span>
                      ))}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          ))}
          {(!fallbacks || fallbacks.length === 0) && (
            <div className="text-center py-16 text-base-content/25 text-sm">No model fallback configs yet</div>
          )}
        </div>
      )}

      <Modal open={createOpen} onClose={() => { setCreateOpen(false); resetForm(); }} title={editId ? 'Edit Model Fallback' : 'Create Model Fallback'}>
        <form onSubmit={handleSave} className="space-y-4">
          <div className="form-control">
            <label className="label"><span className="label-text font-medium">Name</span></label>
            <input type="text" value={name} onChange={(e) => setName(e.target.value)} placeholder="e.g., GPT-4 Fallback Chain" required className="input input-bordered w-full" />
          </div>
          <div className="space-y-4">
            <label className="label"><span className="label-text font-medium">Fallback Groups</span></label>
            <p className="text-xs text-base-content/40 -mt-2">Each group defines equivalent models. Lower priority number = tried first.</p>
            {groups.map((group, gi) => (
              <div key={gi} className="rounded-lg border border-base-300/50 bg-base-200/30 p-3 space-y-2">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-semibold text-base-content/50">Group {gi + 1}</span>
                  {groups.length > 1 && <button type="button" onClick={() => removeGroup(gi)} className="text-error/60 hover:text-error"><X className="h-4 w-4" /></button>}
                </div>
                {group.models.map((model, mi) => (
                  <div key={mi} className="flex gap-2 items-center">
                    <span className="text-xs text-base-content/30 w-6 text-right shrink-0">P{group.priorities[mi]}</span>
                    <input type="text" value={model} onChange={(e) => updateModelName(gi, mi, e.target.value)} placeholder="model-name" className="input input-bordered input-sm flex-1" />
                    <input type="number" value={group.priorities[mi]} onChange={(e) => updatePriority(gi, mi, parseInt(e.target.value) || 1)} min={1} className="input input-bordered input-sm w-16" />
                    {group.models.length > 1 && <button type="button" onClick={() => removeModel(gi, mi)} className="text-base-content/30 hover:text-error shrink-0"><X className="h-4 w-4" /></button>}
                  </div>
                ))}
                <Button variant="ghost" size="sm" type="button" onClick={() => addModel(gi)}>+ Add Model</Button>
              </div>
            ))}
            <Button variant="secondary" size="sm" type="button" onClick={addGroup}>+ Add Group</Button>
          </div>
          <Button variant="primary" loading={createMutation.isPending || updateMutation.isPending}>{editId ? 'Update' : 'Create'}</Button>
        </form>
      </Modal>

      <Modal open={!!deleteId} onClose={() => setDeleteId(null)} title="Delete Model Fallback">
        <p className="text-sm text-base-content/60">Are you sure? API keys referencing this config will lose their fallback chain.</p>
        <div className="mt-4 flex gap-2 justify-end">
          <Button variant="secondary" onClick={() => setDeleteId(null)}>Cancel</Button>
          <Button variant="primary" loading={deleteMutation.isPending} onClick={handleDelete}>Delete</Button>
        </div>
      </Modal>
    </div>
  );
}
