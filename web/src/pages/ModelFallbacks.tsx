import { useState } from 'react';
import { Plus, Trash2, X, ArrowRight, AlertCircle, ArrowRightLeft } from 'lucide-react';
import { motion } from 'framer-motion';
import { useModelFallbacks, useCreateModelFallback, useUpdateModelFallback, useDeleteModelFallback } from '../hooks/useModelFallbacks';
import { useReducedMotion } from '../hooks/useReducedMotion';
import { Button } from '../components/ui/Button';
import { Modal } from '../components/ui/Modal';
import type { ModelFallbackGroup } from '../types';

const EASE = [0.16, 1, 0.3, 1] as const;

export default function ModelFallbacks() {
  const { data: fallbacks, isLoading } = useModelFallbacks();
  const createMutation = useCreateModelFallback();
  const updateMutation = useUpdateModelFallback();
  const deleteMutation = useDeleteModelFallback();
  const reducedMotion = useReducedMotion();

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

  const sortedGroupModels = (group: ModelFallbackGroup) => {
    return group.models
      .map((model, i) => ({ model, priority: group.priorities[i] ?? i + 1 }))
      .sort((a, b) => a.priority - b.priority);
  };

  return (
    <div className="px-6 pb-8">
      {/* Header */}
      <motion.div
        initial={reducedMotion ? false : { opacity: 0, y: -12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={reducedMotion ? { duration: 0 } : { duration: 0.4, ease: EASE }}
        className="mb-8 pt-8 flex items-end justify-between gap-6"
      >
        <div>
          <h1 className="text-3xl font-black tracking-tight text-base-content leading-none mb-1">
            Model Fallbacks
          </h1>
          <p className="text-base text-base-content/50">
            Configure fallback model chains for API keys
          </p>
        </div>
        <Button icon={<Plus className="h-4 w-4" />} onClick={openCreate}>
          Create Fallback
        </Button>
      </motion.div>

      {/* Content */}
      {isLoading ? (
        <div className="space-y-3">
          {[...Array(3)].map((_, i) => (
            <div key={i} className="h-28 bg-base-200/40 rounded-2xl animate-pulse" />
          ))}
        </div>
      ) : (
        <div className="space-y-3">
          {fallbacks?.map((fb, index) => (
            <motion.div
              key={fb.id}
              initial={reducedMotion ? false : { opacity: 0, y: 12 }}
              animate={{ opacity: 1, y: 0 }}
              transition={reducedMotion ? { duration: 0 } : { duration: 0.35, delay: index * 0.05, ease: EASE }}
              className="rounded-2xl border border-base-300/40 bg-base-100 overflow-hidden"
            >
              {/* Card header */}
              <div className="p-5 flex items-center justify-between">
                <div className="flex items-center gap-4">
                  <div className="w-10 h-10 rounded-xl flex items-center justify-center bg-base-200/60 shrink-0">
                    <ArrowRightLeft className="h-5 w-5 text-violet-400" />
                  </div>
                  <div>
                    <h3 className="text-sm font-bold">{fb.name}</h3>
                    <span className="text-xs text-base-content/45">
                      {fb.config.length} group{fb.config.length !== 1 ? 's' : ''}
                    </span>
                  </div>
                </div>
                <div className="flex items-center gap-1">
                  <Button variant="ghost" size="sm" onClick={() => openEdit(fb.id)}>
                    Edit
                  </Button>
                  <button
                    type="button"
                    onClick={() => setDeleteId(fb.id)}
                    className="btn btn-ghost btn-sm text-error/50 hover:text-error cursor-pointer"
                    aria-label={`Delete ${fb.name}`}
                  >
                    <Trash2 className="h-4 w-4" />
                  </button>
                </div>
              </div>

              {/* Fallback groups */}
              <div className="px-5 pb-5 space-y-2">
                {fb.config.map((group, gi) => {
                  const sorted = sortedGroupModels(group);
                  return (
                    <div
                      key={gi}
                      className="flex items-center gap-2 rounded-xl border border-base-300/40 bg-base-100/60 px-4 py-3"
                    >
                      <span className="text-xs font-semibold uppercase tracking-wider text-base-content/40 shrink-0">
                        G{gi + 1}
                      </span>
                      <div className="flex items-center gap-1.5 flex-wrap">
                        {sorted.map((item, mi) => (
                          <span key={mi} className="contents">
                            <span className="inline-flex items-center gap-1.5 rounded-lg bg-base-200/60 px-2.5 py-1 text-xs font-mono">
                              <span className="text-base-content/30 tabular-nums">P{item.priority}</span>
                              <span className="font-medium">{item.model}</span>
                            </span>
                            {mi < sorted.length - 1 && (
                              <ArrowRight className="h-3 w-3 text-base-content/20 shrink-0" />
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
            <motion.div
              initial={reducedMotion ? false : { opacity: 0, y: 12 }}
              animate={{ opacity: 1, y: 0 }}
              transition={reducedMotion ? { duration: 0 } : { duration: 0.35, ease: EASE }}
              className="rounded-2xl border border-base-300/40 bg-base-100 p-5"
            >
              <div className="text-center py-12">
                <div className="w-12 h-12 rounded-xl flex items-center justify-center bg-base-200/60 mx-auto mb-4">
                  <ArrowRightLeft className="h-6 w-6 text-base-content/30" />
                </div>
                <p className="text-sm text-base-content/40 mb-4">
                  No model fallback configs yet
                </p>
                <Button variant="secondary" size="sm" onClick={openCreate}>
                  Create your first fallback
                </Button>
              </div>
            </motion.div>
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
                className="rounded-xl border border-base-300/40 bg-base-100/60 p-3 space-y-2"
              >
                <div className="flex items-center justify-between">
                  <span className="text-xs font-semibold uppercase tracking-wider text-base-content/45">
                    Group {gi + 1}
                  </span>
                  {groups.length > 1 && (
                    <button
                      type="button"
                      onClick={() => removeGroup(gi)}
                      className="text-error/50 hover:text-error cursor-pointer p-1 -m-1"
                      aria-label={`Remove group ${gi + 1}`}
                    >
                      <X className="h-4 w-4" />
                    </button>
                  )}
                </div>
                {group.models.map((model, mi) => (
                  <div key={mi} className="flex gap-2 items-center">
                    <span className="text-xs text-base-content/30 w-6 text-right shrink-0 tabular-nums font-mono">
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
