import { useState } from 'react';
import { Plus, Trash2, X, ArrowRight, AlertCircle } from 'lucide-react';
import { motion } from 'framer-motion';
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
  const [validationError, setValidationError] = useState('');

  const resetForm = () => {
    setName('');
    setGroups([{ models: [''], priorities: [1] }]);
    setEditId(null);
    setValidationError('');
  };
  const openCreate = () => { resetForm(); setCreateOpen(true); };
  const openEdit = (id: string) => {
    const fb = fallbacks?.find((f) => f.id === id);
    if (!fb) return;
    setEditId(id);
    setName(fb.name);
    setGroups(fb.config.length > 0 ? fb.config : [{ models: [''], priorities: [1] }]);
    setValidationError('');
    setCreateOpen(true);
  };

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    setValidationError('');
    const cleanedGroups = groups
      .map((g) => ({
        models: g.models.filter((m) => m.trim() !== ''),
        priorities: g.priorities,
      }))
      .filter((g) => g.models.length >= 2);

    if (cleanedGroups.length === 0) {
      setValidationError('Each group needs at least 2 models to define a fallback chain.');
      return;
    }
    if (editId) {
      await updateMutation.mutateAsync({ id: editId, input: { name, config: cleanedGroups } });
    } else {
      await createMutation.mutateAsync({ name, config: cleanedGroups });
    }
    setCreateOpen(false);
    resetForm();
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
    if (group.models.length === 0) {
      group.models = [''];
      group.priorities = [1];
    }
    updated[gi] = group;
    setGroups(updated);
  };

  const updateModelName = (gi: number, mi: number, value: string) => {
    const updated = [...groups];
    const group = { ...updated[gi] };
    group.models = [...group.models];
    group.models[mi] = value;
    updated[gi] = group;
    setGroups(updated);
  };

  const updatePriority = (gi: number, mi: number, value: number) => {
    const updated = [...groups];
    const group = { ...updated[gi] };
    group.priorities = [...group.priorities];
    group.priorities[mi] = value;
    updated[gi] = group;
    setGroups(updated);
  };

  const handleDelete = async () => {
    if (!deleteId) return;
    await deleteMutation.mutateAsync(deleteId);
    setDeleteId(null);
  };

  // Sort models within each group by priority for display
  const sortedGroupModels = (group: ModelFallbackGroup) => {
    return group.models
      .map((model, i) => ({ model, priority: group.priorities[i] ?? i + 1 }))
      .sort((a, b) => a.priority - b.priority);
  };

  return (
    <div>
      <div className="mb-8 flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Model Fallbacks</h1>
          <p className="text-sm text-base-content/40 mt-1">
            Configure fallback model chains for API keys
          </p>
        </div>
        <Button icon={<Plus className="h-4 w-4" />} onClick={openCreate}>
          Create Fallback
        </Button>
      </div>

      {isLoading ? (
        <div className="flex items-center justify-center py-12">
          <span className="loading loading-spinner loading-lg" />
        </div>
      ) : (
        <div className="space-y-4">
          {fallbacks?.map((fb, index) => (
            <motion.div
              key={fb.id}
              initial={{ opacity: 0, y: 8 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.2, delay: index * 0.05 }}
              className="rounded-xl border border-base-300/50 bg-base-100/60 p-5"
            >
              <div className="flex items-center justify-between mb-3">
                <div>
                  <h3 className="font-semibold text-sm">{fb.name}</h3>
                  <p className="text-xs text-base-content/40 mt-0.5">
                    {fb.config.length} group{fb.config.length !== 1 ? 's' : ''}
                  </p>
                </div>
                <div className="flex gap-1">
                  <Button variant="ghost" size="sm" onClick={() => openEdit(fb.id)}>
                    Edit
                  </Button>
                  <button
                    type="button"
                    onClick={() => setDeleteId(fb.id)}
                    className="btn btn-ghost btn-sm text-error/60 hover:text-error"
                    aria-label={`Delete ${fb.name}`}
                  >
                    <Trash2 className="h-4 w-4" />
                  </button>
                </div>
              </div>
              <div className="space-y-2">
                {fb.config.map((group, gi) => {
                  const sorted = sortedGroupModels(group);
                  return (
                    <div
                      key={gi}
                      className="rounded-lg border border-base-200/50 bg-base-200/30 px-3 py-2.5"
                    >
                      <div className="flex items-center gap-1.5 flex-wrap">
                        {sorted.map((item, mi) => (
                          <span key={mi} className="contents">
                            <span className="inline-flex items-center gap-1.5 rounded-md bg-base-200 px-2.5 py-1 text-xs font-mono">
                              <span className="text-base-content/25 tabular-nums">
                                P{item.priority}
                              </span>
                              <span>{item.model}</span>
                            </span>
                            {mi < sorted.length - 1 && (
                              <ArrowRight className="h-3 w-3 text-base-content/15 shrink-0" />
                            )}
                          </span>
                        ))}
                      </div>
                    </div>
                  );
                })}
              </div>
            </motion.div>
          ))}
          {(!fallbacks || fallbacks.length === 0) && (
            <div className="text-center py-16">
              <p className="text-base-content/25 text-sm mb-4">
                No model fallback configs yet
              </p>
              <Button variant="secondary" size="sm" onClick={openCreate}>
                Create your first fallback
              </Button>
            </div>
          )}
        </div>
      )}

      {/* Create / Edit Modal */}
      <Modal
        open={createOpen}
        onClose={() => {
          setCreateOpen(false);
          resetForm();
        }}
        title={editId ? 'Edit Model Fallback' : 'Create Model Fallback'}
      >
        <form onSubmit={handleSave} className="space-y-4">
          <div className="form-control">
            <label className="label">
              <span className="label-text font-medium">Name</span>
            </label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="e.g., GPT-4 Fallback Chain"
              required
              className="input input-bordered w-full"
            />
          </div>

          <div className="space-y-3">
            <div>
              <label className="label">
                <span className="label-text font-medium">Fallback Groups</span>
              </label>
              <p className="text-xs text-base-content/40 -mt-1">
                Each group defines equivalent models. Lower priority number = tried first.
              </p>
            </div>

            {validationError && (
              <div
                role="alert"
                className="flex items-start gap-2 rounded-lg border border-error/20 bg-error/5 px-3 py-2 text-sm text-error"
              >
                <AlertCircle className="h-4 w-4 shrink-0 mt-0.5" />
                <span>{validationError}</span>
              </div>
            )}

            {groups.map((group, gi) => (
              <div
                key={gi}
                className="rounded-lg border border-base-300/50 bg-base-200/30 p-3 space-y-2"
              >
                <div className="flex items-center justify-between">
                  <span className="text-xs font-semibold text-base-content/50">
                    Group {gi + 1}
                  </span>
                  {groups.length > 1 && (
                    <button
                      type="button"
                      onClick={() => removeGroup(gi)}
                      className="text-error/60 hover:text-error cursor-pointer p-1 -m-1"
                      aria-label={`Remove group ${gi + 1}`}
                    >
                      <X className="h-4 w-4" />
                    </button>
                  )}
                </div>
                {group.models.map((model, mi) => (
                  <div key={mi} className="flex gap-2 items-center">
                    <span className="text-xs text-base-content/30 w-6 text-right shrink-0 tabular-nums">
                      P{group.priorities[mi]}
                    </span>
                    <input
                      type="text"
                      value={model}
                      onChange={(e) => updateModelName(gi, mi, e.target.value)}
                      placeholder="model-name"
                      className="input input-bordered input-sm flex-1"
                    />
                    <input
                      type="number"
                      value={group.priorities[mi]}
                      onChange={(e) =>
                        updatePriority(gi, mi, parseInt(e.target.value) || 1)
                      }
                      min={1}
                      className="input input-bordered input-sm w-16"
                    />
                    {group.models.length > 1 && (
                      <button
                        type="button"
                        onClick={() => removeModel(gi, mi)}
                        className="text-base-content/30 hover:text-error shrink-0 cursor-pointer p-1"
                        aria-label={`Remove model ${model || `P${group.priorities[mi]}`}`}
                      >
                        <X className="h-4 w-4" />
                      </button>
                    )}
                  </div>
                ))}
                <Button
                  variant="ghost"
                  size="sm"
                  type="button"
                  onClick={() => addModel(gi)}
                >
                  + Add Model
                </Button>
              </div>
            ))}
            <Button
              variant="secondary"
              size="sm"
              type="button"
              onClick={addGroup}
            >
              + Add Group
            </Button>
          </div>

          <Button
            variant="primary"
            loading={createMutation.isPending || updateMutation.isPending}
          >
            {editId ? 'Update' : 'Create'}
          </Button>
        </form>
      </Modal>

      {/* Delete Confirmation Modal */}
      <Modal
        open={!!deleteId}
        onClose={() => setDeleteId(null)}
        title="Delete Model Fallback"
      >
        <p className="text-sm text-base-content/60">
          Are you sure? API keys referencing this config will lose their fallback
          chain.
        </p>
        <div className="mt-4 flex gap-2 justify-end">
          <Button variant="secondary" onClick={() => setDeleteId(null)}>
            Cancel
          </Button>
          <Button
            variant="danger"
            loading={deleteMutation.isPending}
            onClick={handleDelete}
          >
            Delete
          </Button>
        </div>
      </Modal>
    </div>
  );
}
