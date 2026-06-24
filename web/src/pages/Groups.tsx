import { useState, useEffect } from 'react';
import { Plus, Pencil, Trash2 } from 'lucide-react';
import { motion } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import { useGroups, useCreateGroup, useUpdateGroup, useDeleteGroup } from '../hooks/useGroups';
import { Button } from '../components/ui/Button';
import { Drawer } from '../components/ui/Drawer';
import { ConfirmDialog } from '../components/ui/ConfirmDialog';
import type { Group } from '../types';

const EASE = [0.16, 1, 0.3, 1] as const;

export default function Groups() {
  const { t } = useTranslation();
  const { data, isLoading } = useGroups();
  const createMutation = useCreateGroup();
  const updateMutation = useUpdateGroup();
  const deleteMutation = useDeleteGroup();

  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');

  const groups = data?.items ?? [];

  const openCreate = () => {
    setEditingId(null);
    setName('');
    setDescription('');
    setDrawerOpen(true);
  };

  const openEdit = (group: Group) => {
    setEditingId(group.id);
    setName(group.name);
    setDescription(group.description ?? '');
    setDrawerOpen(true);
  };

  const closeDrawer = () => {
    setDrawerOpen(false);
    setEditingId(null);
    setName('');
    setDescription('');
  };

  const handleSubmit = () => {
    if (!name.trim()) return;
    if (editingId) {
      updateMutation.mutate(
        {
          id: editingId,
          input: {
            name: name.trim(),
            description: description.trim() || null,
          },
        },
        { onSuccess: closeDrawer },
      );
    } else {
      createMutation.mutate(
        {
          name: name.trim(),
          description: description.trim() || undefined,
        },
        { onSuccess: closeDrawer },
      );
    }
  };

  return (
    <div className="px-6 pb-8">
      {/* Header */}
      <motion.div
        initial={{ opacity: 0, y: -12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.4, ease: EASE }}
        className="mb-8 pt-8 flex items-start justify-between gap-4"
      >
        <div>
          <h1 className="text-3xl font-black tracking-tight text-base-content leading-none mb-1">
            {t('groups.title')}
          </h1>
          <p className="text-base text-base-content/50">
            {t('groups.description')}
          </p>
        </div>
        <Button onClick={openCreate} icon={<Plus className="h-4 w-4" />}>
          {t('groups.addGroup')}
        </Button>
      </motion.div>

      {isLoading ? (
        <div className="flex items-center justify-center py-12">
          <span className="loading loading-spinner loading-lg" />
        </div>
      ) : (
        <motion.div
          initial={{ opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.35, delay: 0.05, ease: EASE }}
          className="overflow-x-auto rounded-2xl border border-base-300/40 bg-base-100"
        >
          <table className="table table-sm">
            <thead>
              <tr className="border-b border-base-300/40">
                <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">
                  {t('groups.table.name')}
                </th>
                <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">
                  {t('groups.table.description')}
                </th>
                <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45 text-right">
                  {t('groups.table.actions')}
                </th>
              </tr>
            </thead>
            <tbody>
              {groups.map((group) => (
                <tr
                  key={group.id}
                  className="border-b border-base-200/40 hover:bg-base-200/20 transition-colors"
                >
                  <td className="font-medium">{group.name}</td>
                  <td className="text-sm text-base-content/55">
                    {group.description || '-'}
                  </td>
                  <td>
                    <div className="flex items-center justify-end gap-1">
                      <Button
                        variant="ghost"
                        size="sm"
                        icon={<Pencil className="h-3.5 w-3.5" />}
                        onClick={() => openEdit(group)}
                      >
                        {t('common.edit')}
                      </Button>
                      <ConfirmDialog
                        title={t('groups.deleteConfirm.title')}
                        onConfirm={() => deleteMutation.mutate(group.id)}
                        okText={t('groups.deleteConfirm.confirm')}
                        variant="danger"
                      >
                        <Button
                          variant="ghost"
                          size="sm"
                          icon={<Trash2 className="h-3.5 w-3.5 text-red-500" />}
                        >
                          {t('common.delete')}
                        </Button>
                      </ConfirmDialog>
                    </div>
                  </td>
                </tr>
              ))}
              {groups.length === 0 && (
                <tr>
                  <td colSpan={3} className="text-center py-12 text-base-content/40 text-sm">
                    {t('groups.noGroups')}
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </motion.div>
      )}

      <Drawer
        open={drawerOpen}
        onClose={closeDrawer}
        title={editingId ? t('groups.editModal.title') : t('groups.createModal.title')}
        width={480}
      >
        <GroupForm
          name={name}
          description={description}
          onNameChange={setName}
          onDescriptionChange={setDescription}
          onSubmit={handleSubmit}
          onCancel={closeDrawer}
          isEditing={editingId !== null}
          isPending={createMutation.isPending || updateMutation.isPending}
        />
      </Drawer>
    </div>
  );
}

function GroupForm({
  name,
  description,
  onNameChange,
  onDescriptionChange,
  onSubmit,
  onCancel,
  isEditing,
  isPending,
}: {
  name: string;
  description: string;
  onNameChange: (v: string) => void;
  onDescriptionChange: (v: string) => void;
  onSubmit: () => void;
  onCancel: () => void;
  isEditing: boolean;
  isPending: boolean;
}) {
  const { t } = useTranslation();

  // Submit on Cmd/Ctrl + Enter
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
        onSubmit();
      }
    };
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  }, [onSubmit]);

  return (
    <div className="space-y-4">
      <div>
        <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-1.5 block">
          {t('groups.createModal.name')}
        </label>
        <input
          type="text"
          value={name}
          onChange={(e) => onNameChange(e.target.value)}
          placeholder={t('groups.createModal.namePlaceholder')}
          autoFocus
          className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm text-base-content focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
        />
      </div>
      <div>
        <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-1.5 block">
          {t('groups.createModal.description')}
        </label>
        <textarea
          value={description}
          onChange={(e) => onDescriptionChange(e.target.value)}
          placeholder={t('groups.createModal.descriptionPlaceholder')}
          rows={4}
          className="w-full rounded-lg border border-base-300 bg-base-200/50 px-3 py-2 text-sm text-base-content focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors resize-none"
        />
      </div>
      <div className="flex justify-end gap-2 pt-2">
        <Button variant="ghost" onClick={onCancel} disabled={isPending}>
          {t('common.cancel')}
        </Button>
        <Button onClick={onSubmit} loading={isPending} disabled={!name.trim()}>
          {isEditing ? t('groups.editModal.saveChanges') : t('groups.createModal.createGroup')}
        </Button>
      </div>
    </div>
  );
}