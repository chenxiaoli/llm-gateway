import { useState } from 'react';
import { motion } from 'framer-motion';
import { UserPlus, Users } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useMembers, useInviteMember, useChangeMemberRole, useRemoveMember } from '../hooks/useMembers';
import { useAuthStore } from '../stores/authStore';
import { isAdminOrAbove } from '../lib/auth';
import { useReducedMotion } from '../hooks/useReducedMotion';
import type { Member, MemberRole } from '../types';
import { Button } from '../components/ui/Button';
import { Modal } from '../components/ui/Modal';
import { Select } from '../components/ui/Select';
import { ConfirmDialog } from '../components/ui/ConfirmDialog';
import { LoadingSpinner } from '../components/ui/LoadingSpinner';

const EASE = [0.16, 1, 0.3, 1] as const;

const ROLE_OPTIONS: MemberRole[] = ['owner', 'admin', 'member'];

function isForbiddenError(err: unknown): boolean {
  const e = err as { response?: { status?: number } };
  return e?.response?.status === 403;
}

export default function Members() {
  const { t } = useTranslation();
  const reducedMotion = useReducedMotion();
  const user = useAuthStore((s) => s.user);
  const currentOrg = useAuthStore((s) => s.currentOrg);
  const { data: members, isLoading, error } = useMembers();
  const inviteMutation = useInviteMember();
  const roleMutation = useChangeMemberRole();
  const removeMutation = useRemoveMember();

  const canManage = isAdminOrAbove(user, currentOrg);

  const [inviteOpen, setInviteOpen] = useState(false);
  const [inviteUsername, setInviteUsername] = useState('');
  const [inviteRole, setInviteRole] = useState<MemberRole>('member');

  const handleInviteSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    inviteMutation.mutate(
      { username: inviteUsername.trim(), role: inviteRole },
      {
        onSuccess: () => {
          setInviteOpen(false);
          setInviteUsername('');
          setInviteRole('member');
        },
      },
    );
  };

  const handleRoleChange = (m: Member, nextRole: MemberRole) => {
    if (nextRole === m.role) return;
    // Confirm before demoting from owner.
    if (m.role === 'owner' && nextRole !== 'owner') {
      const label = t(`members.role.${nextRole}`);
      if (!window.confirm(t('members.confirmDemote', { role: label }))) return;
    }
    roleMutation.mutate({ userId: m.user_id, role: nextRole });
  };

  const handleRemove = (m: Member) => {
    removeMutation.mutate(m.user_id);
    // If the current user leaves, the backend invalidates their membership;
    // the app will re-bootstrap on next navigation.
  };

  const forbidden = !isLoading && error && isForbiddenError(error);

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
            {t('members.title')}
          </h1>
          <p className="text-base text-base-content/50">{t('members.description')}</p>
        </div>
        {canManage && (
          <Button icon={<UserPlus className="h-4 w-4" />} onClick={() => setInviteOpen(true)}>
            {t('members.invite')}
          </Button>
        )}
      </motion.div>

      {isLoading ? (
        <div className="flex items-center justify-center py-12">
          <LoadingSpinner size="lg" />
        </div>
      ) : forbidden ? (
        <div className="rounded-2xl border border-base-300/40 bg-base-100 p-10 text-center">
          <div className="mx-auto mb-3 w-12 h-12 rounded-xl flex items-center justify-center bg-base-200/60">
            <Users className="h-6 w-6 text-base-content/30" />
          </div>
          <p className="text-sm text-base-content/60">{t('members.forbidden')}</p>
        </div>
      ) : (
        <motion.div
          initial={reducedMotion ? false : { opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={reducedMotion ? { duration: 0 } : { duration: 0.35, delay: 0.05, ease: EASE }}
          className="overflow-x-auto rounded-2xl border border-base-300/40 bg-base-100"
        >
          <table className="table table-sm">
            <thead>
              <tr className="border-b border-base-300/40">
                <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">
                  {t('members.table.username')}
                </th>
                <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">
                  {t('members.table.role')}
                </th>
                <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">
                  {t('members.table.joined')}
                </th>
                <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">
                  {t('members.table.actions')}
                </th>
              </tr>
            </thead>
            <tbody>
              {members?.map((m) => {
                const isSelf = m.user_id === user?.id;
                return (
                  <tr key={m.user_id} className="border-b border-base-200/40 hover:bg-base-200/20 transition-colors">
                    <td>
                      <span className="font-medium text-base-content/90">
                        {m.username}
                        {isSelf && (
                          <span className="ml-2 text-xs text-base-content/40">{t('members.selfLabel')}</span>
                        )}
                      </span>
                    </td>
                    <td>
                      {canManage ? (
                        <Select
                          value={m.role}
                          size="sm"
                          onChange={(value) => handleRoleChange(m, value as MemberRole)}
                          options={ROLE_OPTIONS.map((r) => ({ value: r, label: t(`members.role.${r}`) }))}
                        />
                      ) : (
                        <span className="text-sm text-base-content/70">{t(`members.role.${m.role}`)}</span>
                      )}
                    </td>
                    <td className="font-mono text-sm text-base-content/55">
                      {new Date(m.joined_at).toLocaleDateString()}
                    </td>
                    <td>
                      {/* Self-leave is allowed for any member; removing others requires admin+. */}
                      {isSelf ? (
                        <ConfirmDialog
                          title={t('members.confirmLeave')}
                          okText={t('members.leave')}
                          variant="danger"
                          onConfirm={() => handleRemove(m)}
                        >
                          <Button variant="danger" size="sm">{t('members.leave')}</Button>
                        </ConfirmDialog>
                      ) : canManage ? (
                        <ConfirmDialog
                          title={t('members.confirmRemove', { name: m.username })}
                          okText={t('common.remove')}
                          variant="danger"
                          onConfirm={() => handleRemove(m)}
                        >
                          <Button variant="danger" size="sm">{t('members.remove')}</Button>
                        </ConfirmDialog>
                      ) : null}
                    </td>
                  </tr>
                );
              })}
              {members?.length === 0 && (
                <tr>
                  <td colSpan={4} className="text-center py-12 text-base-content/40 text-sm">
                    {t('members.noMembers')}
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </motion.div>
      )}

      {/* Invite Modal */}
      <Modal open={inviteOpen} onClose={() => setInviteOpen(false)} title={t('members.inviteModal.title')}>
        <form onSubmit={handleInviteSubmit} className="space-y-4">
          <div>
            <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-1.5 block">
              {t('members.inviteModal.username')}
            </label>
            <input
              type="text"
              value={inviteUsername}
              onChange={(e) => setInviteUsername(e.target.value)}
              placeholder={t('members.inviteModal.usernamePlaceholder')}
              required
              autoFocus
              className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm text-base-content focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
            />
          </div>
          <div>
            <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-1.5 block">
              {t('members.inviteModal.role')}
            </label>
            <Select
              value={inviteRole}
              onChange={(value) => setInviteRole(value as MemberRole)}
              options={ROLE_OPTIONS.map((r) => ({ value: r, label: t(`members.role.${r}`) }))}
            />
          </div>
          <div className="flex justify-end gap-2 pt-2">
            <Button variant="ghost" type="button" onClick={() => setInviteOpen(false)}>
              {t('common.cancel')}
            </Button>
            <Button type="submit" loading={inviteMutation.isPending}>
              {t('members.inviteModal.invite')}
            </Button>
          </div>
        </form>
      </Modal>
    </div>
  );
}
