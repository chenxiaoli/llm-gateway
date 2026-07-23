import { useState } from 'react';
import { Plus, Trash2, AlertCircle, Compass, Check } from 'lucide-react';
import { motion } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import {
  useAutoRouteConfigs,
  useCreateAutoRouteConfig,
  useUpdateAutoRouteConfig,
  useDeleteAutoRouteConfig,
} from '../hooks/useAutoRouteConfigs';
import { useAllModels } from '../hooks/useModels';
import { useReducedMotion } from '../hooks/useReducedMotion';
import { Button } from '../components/ui/Button';
import { Modal } from '../components/ui/Modal';

const EASE = [0.16, 1, 0.3, 1] as const;

export default function AutoRoutes() {
  const { t } = useTranslation();
  const { data: configs, isLoading } = useAutoRouteConfigs();
  const { data: models } = useAllModels();
  const createMutation = useCreateAutoRouteConfig();
  const updateMutation = useUpdateAutoRouteConfig();
  const deleteMutation = useDeleteAutoRouteConfig();
  const reducedMotion = useReducedMotion();

  const [createOpen, setCreateOpen] = useState(false);
  const [editId, setEditId] = useState<string | null>(null);
  const [name, setName] = useState('');
  const [selectedModels, setSelectedModels] = useState<string[]>([]);
  const [deleteId, setDeleteId] = useState<string | null>(null);
  const [validationError, setValidationError] = useState('');

  const resetForm = () => {
    setName('');
    setSelectedModels([]);
    setEditId(null);
    setValidationError('');
  };
  const openCreate = () => { resetForm(); setCreateOpen(true); };
  const openEdit = (id: string) => {
    const c = configs?.find((x) => x.id === id);
    if (!c) return;
    setEditId(id);
    setName(c.name);
    setSelectedModels(c.config.model_names);
    setValidationError('');
    setCreateOpen(true);
  };

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    setValidationError('');
    if (!name.trim()) {
      setValidationError(t('autoRoutes.editModal.nameRequired'));
      return;
    }
    if (selectedModels.length === 0) {
      setValidationError(t('autoRoutes.editModal.atLeastOneModel'));
      return;
    }
    if (editId) {
      await updateMutation.mutateAsync({
        id: editId,
        input: { name, config: { model_names: selectedModels } },
      });
    } else {
      await createMutation.mutateAsync({
        name,
        config: { model_names: selectedModels },
      });
    }
    setCreateOpen(false);
    resetForm();
  };

  const toggleModel = (modelName: string) => {
    setSelectedModels((prev) =>
      prev.includes(modelName) ? prev.filter((m) => m !== modelName) : [...prev, modelName],
    );
  };

  const handleDelete = async () => {
    if (!deleteId) return;
    await deleteMutation.mutateAsync(deleteId);
    setDeleteId(null);
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
            {t('autoRoutes.title')}
          </h1>
          <p className="text-base text-base-content/50">
            {t('autoRoutes.subtitle')}
          </p>
        </div>
        <Button icon={<Plus className="h-4 w-4" />} onClick={openCreate}>
          {t('autoRoutes.createBtn')}
        </Button>
      </motion.div>

      {/* Content */}
      {isLoading ? (
        <div className="space-y-3">
          {[...Array(3)].map((_, i) => (
            <div key={i} className="h-24 bg-base-200/40 rounded-2xl animate-pulse" />
          ))}
        </div>
      ) : (
        <div className="space-y-3">
          {configs?.map((cfg, index) => (
            <motion.div
              key={cfg.id}
              initial={reducedMotion ? false : { opacity: 0, y: 12 }}
              animate={{ opacity: 1, y: 0 }}
              transition={reducedMotion ? { duration: 0 } : { duration: 0.35, delay: index * 0.05, ease: EASE }}
              className="rounded-2xl border border-base-300/40 bg-base-100 overflow-hidden"
            >
              <div className="p-5 flex items-center justify-between">
                <div className="flex items-center gap-4">
                  <div className="w-10 h-10 rounded-xl flex items-center justify-center bg-base-200/60 shrink-0">
                    <Compass className="h-5 w-5 text-sky-400" />
                  </div>
                  <div>
                    <h3 className="text-sm font-bold">{cfg.name}</h3>
                    <span className="text-xs text-base-content/45">
                      {cfg.config.model_names.length} {cfg.config.model_names.length === 1 ? 'model' : 'models'}
                    </span>
                  </div>
                </div>
                <div className="flex items-center gap-1">
                  <Button variant="ghost" size="sm" onClick={() => openEdit(cfg.id)}>
                    {t('autoRoutes.edit')}
                  </Button>
                  <button
                    type="button"
                    onClick={() => setDeleteId(cfg.id)}
                    className="btn btn-ghost btn-sm text-error/50 hover:text-error cursor-pointer"
                    aria-label={t('autoRoutes.deleteLabel', { name: cfg.name })}
                  >
                    <Trash2 className="h-4 w-4" />
                  </button>
                </div>
              </div>

              <div className="px-5 pb-5">
                <div className="flex items-center gap-1.5 flex-wrap">
                  {cfg.config.model_names.map((m, i) => (
                    <span
                      key={i}
                      className="inline-flex items-center gap-1.5 rounded-lg bg-base-200/60 px-2.5 py-1 text-xs font-mono"
                    >
                      <span className="font-medium">{m}</span>
                    </span>
                  ))}
                </div>
              </div>
            </motion.div>
          ))}

          {(!configs || configs.length === 0) && (
            <motion.div
              initial={reducedMotion ? false : { opacity: 0, y: 12 }}
              animate={{ opacity: 1, y: 0 }}
              transition={reducedMotion ? { duration: 0 } : { duration: 0.35, ease: EASE }}
              className="rounded-2xl border border-base-300/40 bg-base-100 p-5"
            >
              <div className="text-center py-12">
                <div className="w-12 h-12 rounded-xl flex items-center justify-center bg-base-200/60 mx-auto mb-4">
                  <Compass className="h-6 w-6 text-base-content/30" />
                </div>
                <p className="text-sm text-base-content/40 mb-4">
                  {t('autoRoutes.empty')}
                </p>
                <Button variant="secondary" size="sm" onClick={openCreate}>
                  {t('autoRoutes.createFirst')}
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
        title={editId ? t('autoRoutes.editModal.editTitle') : t('autoRoutes.editModal.createTitle')}
      >
        <form onSubmit={handleSave} className="space-y-4">
          <div className="form-control">
            <label className="label">
              <span className="label-text font-medium">{t('autoRoutes.editModal.name')}</span>
            </label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder={t('autoRoutes.editModal.namePlaceholder')}
              required
              className="input input-bordered w-full"
            />
          </div>

          <div className="space-y-3">
            <div>
              <label className="label">
                <span className="label-text font-medium">{t('autoRoutes.editModal.models')}</span>
              </label>
              <p className="text-xs text-base-content/40 -mt-1">
                {t('autoRoutes.editModal.modelsDesc')}
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

            <div className="rounded-xl border border-base-300/40 bg-base-100/60 p-3 max-h-72 overflow-y-auto space-y-1">
              {(models ?? []).length === 0 && (
                <p className="text-xs text-base-content/40 px-2 py-4 text-center">
                  {t('autoRoutes.editModal.noModels')}
                </p>
              )}
              {(models ?? []).map((m) => {
                const checked = selectedModels.includes(m.name);
                return (
                  <button
                    key={m.id}
                    type="button"
                    onClick={() => toggleModel(m.name)}
                    className={`w-full flex items-center gap-3 rounded-lg px-3 py-2 text-left cursor-pointer transition-colors ${
                      checked ? 'bg-primary/10 text-primary' : 'hover:bg-base-200/60 text-base-content/80'
                    }`}
                  >
                    <div
                      className={`w-4 h-4 rounded border flex items-center justify-center shrink-0 ${
                        checked ? 'bg-primary border-primary' : 'border-base-content/30'
                      }`}
                    >
                      {checked && <Check className="h-3 w-3 text-primary-content" />}
                    </div>
                    <span className="text-sm font-mono">{m.name}</span>
                  </button>
                );
              })}
            </div>
          </div>

          <Button
            variant="primary"
            loading={createMutation.isPending || updateMutation.isPending}
          >
            {editId ? t('autoRoutes.editModal.save') : t('autoRoutes.editModal.create')}
          </Button>
        </form>
      </Modal>

      {/* Delete Confirmation Modal */}
      <Modal
        open={!!deleteId}
        onClose={() => setDeleteId(null)}
        title={t('autoRoutes.deleteConfirm.title')}
      >
        <p className="text-sm text-base-content/60">
          {t('autoRoutes.deleteConfirm.message')}
        </p>
        <div className="mt-4 flex gap-2 justify-end">
          <Button variant="secondary" onClick={() => setDeleteId(null)}>
            {t('common.cancel')}
          </Button>
          <Button
            variant="danger"
            loading={deleteMutation.isPending}
            onClick={handleDelete}
          >
            {t('autoRoutes.deleteConfirm.confirm')}
          </Button>
        </div>
      </Modal>
    </div>
  );
}
