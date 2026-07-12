import { useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';
import { Mail, Ticket } from 'lucide-react';
import { toast } from 'sonner';
import { listInvitations, createInvitation, revokeInvitation } from '../api/invitations';
import { useAuthStore } from '../stores/authStore';
import { useReducedMotion } from '../hooks/useReducedMotion';
import { Button } from '../components/ui/Button';
import { Select } from '../components/ui/Select';
import { ConfirmDialog } from '../components/ui/ConfirmDialog';
import { LoadingSpinner } from '../components/ui/LoadingSpinner';
import { CopyableInviteLink } from '../components/CopyableInviteLink';
import { getErrorMessage } from '../api/client';
import type { Invitation } from '../types';

const EASE = [0.16, 1, 0.3, 1] as const;

type InviteRole = 'member' | 'admin';

// Minimal client-side email shape check. The backend does the rigorous
// validation; this just keeps the Generate button disabled until the field
// looks plausibly like an email so we don't fire obviously-bad requests.
const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

export default function Invitations() {
  // Source org slug from the auth store (matches useMembers pattern) rather
  // than route params so the page is testable without a real Router match.
  const orgSlug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  const { t } = useTranslation();
  const reducedMotion = useReducedMotion();
  const qc = useQueryClient();

  const [role, setRole] = useState<InviteRole>('member');
  const [recipientEmail, setRecipientEmail] = useState('');
  const [pendingRevoke, setPendingRevoke] = useState<Invitation | null>(null);

  const queryKey = ['invitations', orgSlug];
  const { data: invitations, isLoading } = useQuery({
    queryKey,
    queryFn: () => listInvitations(orgSlug),
    enabled: !!orgSlug,
  });

  const emailValid = EMAIL_RE.test(recipientEmail.trim());

  const createMut = useMutation({
    mutationFn: () => createInvitation(orgSlug, { role, recipient_email: recipientEmail.trim() }),
    onSuccess: () => {
      toast.success(t('invitations.toasts.createdTo', { email: recipientEmail.trim() }));
      setRecipientEmail('');
      qc.invalidateQueries({ queryKey });
    },
    onError: (err) => {
      const code = (err as { response?: { status?: number; data?: { error?: { code?: string } } } })
        ?.response?.data?.error?.code;
      const status = (err as { response?: { status?: number } })?.response?.status;
      if (status === 409 && code === 'email_in_use') {
        toast.error(t('invitations.errors.emailInUse'));
      } else {
        toast.error(getErrorMessage(err, t('invitations.toasts.createFailed')));
      }
    },
  });

  const revokeMut = useMutation({
    mutationFn: (id: string) => revokeInvitation(orgSlug, id),
    onSuccess: () => {
      toast.success(t('invitations.toasts.revoked'));
      qc.invalidateQueries({ queryKey });
    },
    onError: () => toast.error(t('invitations.toasts.revokeFailed')),
  });

  const confirmRevoke = () => {
    if (!pendingRevoke) return;
    revokeMut.mutate(pendingRevoke.id);
    setPendingRevoke(null);
  };

  const statusOf = (inv: Invitation): 'pending' | 'accepted' | 'revoked' | 'expired' => {
    if (inv.accepted_at) return 'accepted';
    if (inv.revoked_at) return 'revoked';
    if (new Date(inv.expires_at).getTime() <= Date.now()) return 'expired';
    return 'pending';
  };

  const roleOptions: { value: InviteRole; label: string }[] = [
    { value: 'member', label: t('invitations.roles.member') },
    { value: 'admin', label: t('invitations.roles.admin') },
  ];

  return (
    <div className="px-6 pb-8">
      {/* Header */}
      <motion.div
        initial={reducedMotion ? false : { opacity: 0, y: -12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={reducedMotion ? { duration: 0 } : { duration: 0.4, ease: EASE }}
        className="mb-8 pt-8"
      >
        <h1 className="text-3xl font-black tracking-tight text-base-content leading-none mb-1">
          {t('invitations.title')}
        </h1>
        <p className="text-base text-base-content/50">{t('invitations.description')}</p>
      </motion.div>

      {/* Generate */}
      <motion.div
        initial={reducedMotion ? false : { opacity: 0, y: 12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={reducedMotion ? { duration: 0 } : { duration: 0.35, delay: 0.05, ease: EASE }}
        className="rounded-2xl border border-base-300/40 bg-base-100 p-5 mb-6"
      >
        <div className="flex items-center gap-2 mb-4">
          <Ticket className="h-4 w-4 text-base-content/50" />
          <h2 className="text-sm font-semibold text-base-content">{t('invitations.generate.title')}</h2>
        </div>
        <div className="flex flex-col sm:flex-row sm:items-end gap-3">
          <div className="flex-1 max-w-xs">
            <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-1.5 block">
              {t('invitations.generate.role')}
            </label>
            <Select
              value={role}
              onChange={(value) => setRole(value as InviteRole)}
              options={roleOptions}
            />
          </div>
          <div className="flex-1 max-w-xs">
            <label
              htmlFor="invitation-recipient-email"
              className="text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-1.5 block"
            >
              {t('invitations.generate.recipientEmail')}
            </label>
            <input
              id="invitation-recipient-email"
              type="email"
              value={recipientEmail}
              onChange={(e) => setRecipientEmail(e.target.value)}
              placeholder={t('invitations.generate.recipientEmailPlaceholder')}
              required
              autoComplete="email"
              className="input input-bordered w-full"
            />
          </div>
          <Button
            type="button"
            onClick={() => createMut.mutate()}
            loading={createMut.isPending}
            disabled={!emailValid}
          >
            {t('invitations.generate.submit')}
          </Button>
        </div>
      </motion.div>

      {/* List */}
      {isLoading ? (
        <div className="flex items-center justify-center py-12">
          <LoadingSpinner size="lg" />
        </div>
      ) : invitations && invitations.length > 0 ? (
        <motion.div
          initial={reducedMotion ? false : { opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={reducedMotion ? { duration: 0 } : { duration: 0.35, delay: 0.1, ease: EASE }}
          className="rounded-2xl border border-base-300/40 bg-base-100 overflow-x-auto"
        >
          <table className="table table-sm">
            <thead>
              <tr className="border-b border-base-300/40">
                <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">
                  {t('invitations.table.link')}
                </th>
                <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">
                  {t('invitations.table.role')}
                </th>
                <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">
                  {t('invitations.table.recipient')}
                </th>
                <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">
                  {t('invitations.table.status')}
                </th>
                <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">
                  {t('invitations.table.created')}
                </th>
                <th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">
                  {t('invitations.table.actions')}
                </th>
              </tr>
            </thead>
            <tbody>
              {invitations.map((inv) => {
                const status = statusOf(inv);
                const isPending = status === 'pending';
                return (
                  <tr key={inv.id} className="border-b border-base-200/40 hover:bg-base-200/20 transition-colors align-middle">
                    <td className="max-w-[20rem]">
                      {isPending ? (
                        <CopyableInviteLink url={inv.url} expiresAt={inv.expires_at} />
                      ) : status === 'accepted' ? (
                        <span className="text-sm text-base-content/70">
                          {inv.accepted_by
                            ? t('invitations.acceptedBy', { user: inv.accepted_by })
                            : t('invitations.acceptedByUnknown')}
                        </span>
                      ) : (
                        <code className="text-xs bg-base-200/60 px-2 py-1 rounded truncate max-w-[18rem] block text-base-content/40">
                          {inv.url}
                        </code>
                      )}
                    </td>
                    <td>
                      <span className="text-sm text-base-content/70">{t(`invitations.roles.${inv.role}`)}</span>
                    </td>
                    <td>
                      {inv.recipient_email ? (
                        <code className="text-xs font-mono text-base-content/60 break-all">
                          {inv.recipient_email}
                        </code>
                      ) : (
                        <span className="text-xs text-base-content/30">—</span>
                      )}
                    </td>
                    <td>
                      <span className={
                        status === 'pending'
                          ? 'text-xs font-medium px-2 py-0.5 rounded-full bg-warning/15 text-warning-content/80'
                          : status === 'accepted'
                            ? 'text-xs font-medium px-2 py-0.5 rounded-full bg-success/15 text-success-content/80'
                            : 'text-xs font-medium px-2 py-0.5 rounded-full bg-base-200 text-base-content/50'
                      }>
                        {t(`invitations.status.${status}`)}
                      </span>
                    </td>
                    <td className="font-mono text-sm text-base-content/55">
                      {new Date(inv.created_at).toLocaleDateString()}
                    </td>
                    <td>
                      {isPending && (
                        <Button
                          type="button"
                          variant="danger"
                          size="sm"
                          onClick={() => setPendingRevoke(inv)}
                        >
                          {t('invitations.revoke')}
                        </Button>
                      )}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </motion.div>
      ) : (
        <motion.div
          initial={reducedMotion ? false : { opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={reducedMotion ? { duration: 0 } : { duration: 0.35, delay: 0.1, ease: EASE }}
          className="rounded-2xl border border-base-300/40 bg-base-100 p-10 text-center"
        >
          <div className="mx-auto mb-3 w-12 h-12 rounded-xl flex items-center justify-center bg-base-200/60">
            <Mail className="h-6 w-6 text-base-content/30" />
          </div>
          <p className="text-sm text-base-content/60">{t('invitations.empty')}</p>
        </motion.div>
      )}

      {/* Revoke confirmation */}
      <ConfirmDialog
        open={pendingRevoke !== null}
        title={t('invitations.confirmRevoke')}
        okText={t('invitations.revoke')}
        variant="danger"
        onConfirm={confirmRevoke}
        onCancel={() => setPendingRevoke(null)}
      >
        {t('invitations.confirmRevokeDescription')}
      </ConfirmDialog>
    </div>
  );
}
