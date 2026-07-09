import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { motion } from 'framer-motion';
import { AlertTriangle, Building2 } from 'lucide-react';
import { toast } from 'sonner';
import { useTranslation } from 'react-i18next';
import { useAuthStore } from '../stores/authStore';
import { isAdminOrAbove } from '../lib/auth';
import { useReducedMotion } from '../hooks/useReducedMotion';
import { updateOrg, deleteOrg } from '../api/orgs';
import { getErrorMessage } from '../api/client';
import { Button } from '../components/ui/Button';
import { ConfirmDialog } from '../components/ui/ConfirmDialog';
import { useGetOrgDefaults, useUpdateOrgDefaults } from '../hooks/useOrgDefaults';

const EASE = [0.16, 1, 0.3, 1] as const;

const INPUT_CLASS =
  'w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm text-base-content focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors disabled:opacity-60 disabled:cursor-not-allowed';

export default function OrgSettings() {
  const { t } = useTranslation();
  const reducedMotion = useReducedMotion();
  const navigate = useNavigate();
  const user = useAuthStore((s) => s.user);
  const currentOrg = useAuthStore((s) => s.currentOrg);
  const setCurrentOrg = useAuthStore((s) => s.setCurrentOrg);
  const logout = useAuthStore((s) => s.logout);

  // General section — admin+ can edit. We always render the inputs so members
  // can see the current values; members get disabled inputs + no Save button
  // (per the "read-only" option in the task spec).
  const canEdit = isAdminOrAbove(user, currentOrg);
  // Owner (or platform_admin) can delete the org.
  const canDelete =
    currentOrg?.role === 'owner' || user?.platform_role === 'platform_admin';

  const [name, setName] = useState(currentOrg?.name ?? '');
  const [slug, setSlug] = useState(currentOrg?.slug ?? '');
  const [saving, setSaving] = useState(false);

  // Danger zone — type-slug-to-confirm + password.
  const [deleteConfirm, setDeleteConfirm] = useState('');
  const [password, setPassword] = useState('');
  const [deleting, setDeleting] = useState(false);
  const [confirmOpen, setConfirmOpen] = useState(false);

  if (!currentOrg) {
    // OrgRouteGuard should prevent this; render nothing to keep TS happy.
    return null;
  }

  const nameChanged = name.trim() !== currentOrg.name;
  const slugChanged = slug.trim() !== currentOrg.slug && slug.trim().length > 0;
  const generalDirty = canEdit && (nameChanged || slugChanged);

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!currentOrg || !generalDirty) return;

    const nextName = name.trim();
    const nextSlug = slug.trim();
    const prevSlug = currentOrg.slug;

    // Mirror backend validation up-front so we get a snappy error without
    // a round-trip on obviously-invalid input.
    if (!nextName) {
      toast.error(t('orgSettings.toasts.updateFailed'));
      return;
    }
    if (slugChanged) {
      if (nextSlug.length < 3 || nextSlug.length > 64) {
        toast.error(t('orgSettings.toasts.updateFailed'));
        return;
      }
      if (!/^[a-z0-9-]+$/.test(nextSlug)) {
        toast.error(t('orgSettings.toasts.updateFailed'));
        return;
      }
    }

    setSaving(true);
    try {
      const updated = await updateOrg({
        name: nameChanged ? nextName : undefined,
        slug: slugChanged ? nextSlug : undefined,
      });
      // After a slug rename, the JWT in localStorage still encodes the old
      // org context — so we route through setCurrentOrg (which POSTs to
      // /auth/select-org, rotates the token, persists it, and clears the
      // React Query cache) rather than just setState-ing currentOrg. The
      // extra round-trip is the price of keeping the token consistent with
      // the new slug. Role is unchanged per the backend contract.
      await setCurrentOrg({
        id: updated.id,
        slug: updated.slug,
        name: updated.name,
        role: updated.role,
        group_id: updated.group_id,
      });
      // setCurrentOrg only updates `currentOrg`; patch the orgs list too so
      // OrgRouteGuard and OrgSwitcher recognise the new slug. Without this,
      // navigating to /${newSlug}/settings would redirect because the stale
      // `orgs` array still holds the old slug.
      const prevOrgs = useAuthStore.getState().orgs;
      useAuthStore.setState({
        orgs: prevOrgs.map((o) =>
          o.id === updated.id
            ? {
                id: updated.id,
                slug: updated.slug,
                name: updated.name,
                role: updated.role,
                group_id: updated.group_id,
              }
            : o,
        ),
      });
      toast.success(t('orgSettings.toasts.updated'));
      // If the slug changed, the current URL is stale — navigate to the new
      // org-scoped settings route. Name-only changes don't need navigation.
      if (slugChanged && updated.slug !== prevSlug) {
        navigate(`/${updated.slug}/settings`, { replace: true });
      }
    } catch (err) {
      const status = (err as { response?: { status?: number } })?.response?.status;
      // 400 (empty/invalid), 409 (duplicate slug) → toast the server message.
      // Anything else also falls through to a generic failure toast.
      if (status === 400 || status === 409) {
        toast.error(
          getErrorMessage(err, t('orgSettings.toasts.updateFailed')),
        );
      } else {
        toast.error(t('orgSettings.toasts.updateFailed'));
      }
    } finally {
      setSaving(false);
    }
  };

  const openDeleteConfirm = (e: React.FormEvent) => {
    e.preventDefault();
    if (deleteConfirm !== currentOrg.slug || !password) return;
    setConfirmOpen(true);
  };

  const handleDelete = async () => {
    if (!currentOrg) return;
    setDeleting(true);
    try {
      await deleteOrg(password);
      toast.success(t('orgSettings.toasts.deleted'));
      // The user's membership is gone and their token is invalidated by the
      // cascade. logout() clears local auth state and redirects to /login.
      logout();
    } catch (err) {
      const status = (err as { response?: { status?: number } })?.response?.status;
      if (status === 401) {
        toast.error(t('orgSettings.toasts.wrongPassword'));
      } else {
        toast.error(t('orgSettings.toasts.deleteFailed'));
      }
    } finally {
      setDeleting(false);
      setConfirmOpen(false);
    }
  };

  const anim = (delay = 0) =>
    reducedMotion
      ? false
      : { initial: { opacity: 0, y: 12 }, animate: { opacity: 1, y: 0 }, transition: { duration: 0.35, delay, ease: EASE } };

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
          {t('orgSettings.title')}
        </h1>
        <p className="text-base text-base-content/50">{t('orgSettings.description')}</p>
      </motion.div>

      <div className="max-w-2xl space-y-6">
        {/* General section — visible to all members, editable by admin+. */}
        <motion.section
          {...anim(0.05)}
          className="rounded-2xl border border-base-300/40 bg-base-100 p-6"
        >
          <h2 className="text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-4">
            {t('orgSettings.general.sectionTitle')}
          </h2>
          <form onSubmit={handleSave} className="space-y-4">
            <div>
              <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-1.5 block">
                {t('orgSettings.general.name')}
              </label>
              <input
                type="text"
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder={t('orgSettings.general.namePlaceholder')}
                disabled={!canEdit || saving}
                autoComplete="off"
                className={INPUT_CLASS}
              />
            </div>
            <div>
              <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-1.5 block">
                {t('orgSettings.general.slug')}
              </label>
              <input
                type="text"
                value={slug}
                onChange={(e) => setSlug(e.target.value)}
                placeholder={t('orgSettings.general.slugHint')}
                disabled={!canEdit || saving}
                autoComplete="off"
                spellCheck={false}
                className={`${INPUT_CLASS} font-mono`}
              />
              <p className="mt-1.5 text-xs text-base-content/40">
                {t('orgSettings.general.slugHint')}
              </p>
            </div>

            <div className="flex items-center justify-between gap-3 pt-1">
              {!canEdit && (
                <span className="text-xs text-base-content/40">
                  {t('orgSettings.general.readOnlyNotice')}
                </span>
              )}
              {canEdit && (
                <Button type="submit" loading={saving} disabled={!generalDirty}>
                  {t('orgSettings.general.save')}
                </Button>
              )}
            </div>
          </form>
        </motion.section>

        {/* Defaults section — admin can edit; member is read-only. */}
        <DefaultsSection canEdit={canEdit} />

        {/* Danger zone — owner-only. Hidden entirely for non-owners. */}
        {canDelete && (
          <motion.section
            {...anim(0.1)}
            className="rounded-2xl border border-red-500/30 bg-red-500/[0.03] p-6"
          >
            <h2 className="text-xs font-semibold uppercase tracking-wider text-red-500/80 mb-4 flex items-center gap-1.5">
              <AlertTriangle className="h-3.5 w-3.5" />
              {t('orgSettings.danger.sectionTitle')}
            </h2>

            <div className="mb-5 flex gap-2.5 rounded-lg bg-red-500/[0.06] border border-red-500/20 px-3.5 py-3">
              <AlertTriangle className="h-4 w-4 text-red-500/70 shrink-0 mt-0.5" />
              <p className="text-sm text-base-content/70 leading-relaxed">
                {t('orgSettings.danger.warning')}
              </p>
            </div>

            <form onSubmit={openDeleteConfirm} className="space-y-4">
              <div>
                <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-1.5 block">
                  {t('orgSettings.danger.confirmLabel')}
                </label>
                <input
                  type="text"
                  value={deleteConfirm}
                  onChange={(e) => setDeleteConfirm(e.target.value)}
                  placeholder={currentOrg.slug}
                  disabled={deleting}
                  autoComplete="off"
                  spellCheck={false}
                  className={`${INPUT_CLASS} font-mono`}
                />
              </div>
              <div>
                <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-1.5 block">
                  {t('orgSettings.danger.passwordLabel')}
                </label>
                <input
                  type="password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  placeholder={t('orgSettings.danger.passwordPlaceholder')}
                  disabled={deleting}
                  autoComplete="current-password"
                  className={INPUT_CLASS}
                />
              </div>
              <div className="pt-1">
                <Button
                  type="submit"
                  variant="danger"
                  loading={deleting}
                  disabled={deleteConfirm !== currentOrg.slug || !password}
                  icon={<Building2 className="h-4 w-4" />}
                >
                  {t('orgSettings.danger.delete')}
                </Button>
              </div>
            </form>
          </motion.section>
        )}
      </div>

      {/* Delete confirmation — extra friction for an irreversible action.
          Title is the load-bearing label here; the user already typed the
          slug + password in the form, this just guards the final click. */}
      <ConfirmDialog
        open={confirmOpen}
        title={t('orgSettings.danger.deleteConfirm')}
        okText={t('orgSettings.danger.delete')}
        variant="danger"
        onConfirm={handleDelete}
        onCancel={() => setConfirmOpen(false)}
      />
    </div>
  );
}

function DefaultsSection({ canEdit }: { canEdit: boolean }) {
  const { t } = useTranslation();
  const reducedMotion = useReducedMotion();
  const { data, isLoading, isError } = useGetOrgDefaults();
  const updateDefaults = useUpdateOrgDefaults();

  // Local state mirrors the loaded values; initialized once data arrives.
  const [rpm, setRpm] = useState<string>('');
  const [budget, setBudget] = useState<string>('');
  const [hydrated, setHydrated] = useState(false);

  useEffect(() => {
    if (data && (!hydrated || updateDefaults.isSuccess)) {
      setRpm(data.default_rate_limit_rpm?.toString() ?? '');
      setBudget(data.default_budget_monthly_usd?.toString() ?? '');
      setHydrated(true);
    }
  }, [data, hydrated, updateDefaults.isSuccess]);

  if (isLoading) {
    return (
      <motion.section
        initial={reducedMotion ? false : { opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.3, ease: EASE }}
        className="rounded-xl border border-base-300 bg-base-100 p-6 mt-6"
      >
        <div className="text-base-content/60">{t('orgSettings.defaults.loading')}</div>
      </motion.section>
    );
  }

  if (isError) {
    return (
      <motion.section
        initial={reducedMotion ? false : { opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.3, ease: EASE }}
        className="rounded-xl border border-base-300 bg-base-100 p-6 mt-6"
      >
        <div className="text-error">{t('orgSettings.defaults.loadError')}</div>
      </motion.section>
    );
  }

  const rpmValid = rpm === '' || /^\d+$/.test(rpm);
  const budgetValid = budget === '' || /^\d+(\.\d+)?$/.test(budget);
  const rpmNum = rpmValid && rpm !== '' ? parseInt(rpm, 10) : null;
  const budgetNum = budgetValid && budget !== '' ? parseFloat(budget) : null;
  const dirty =
    rpmNum !== (data?.default_rate_limit_rpm ?? null) ||
    budgetNum !== (data?.default_budget_monthly_usd ?? null);
  const canSave = dirty && rpmValid && budgetValid && !updateDefaults.isPending;

  const onSave = async () => {
    try {
      await updateDefaults.mutateAsync({
        default_rate_limit_rpm: rpmNum !== null && !Number.isNaN(rpmNum) ? rpmNum : null,
        default_budget_monthly_usd: budgetNum !== null && !Number.isNaN(budgetNum) ? budgetNum : null,
      });
    } catch {
      // error toast already shown by useUpdateOrgDefaults.onError
    }
  };

  return (
    <motion.section
      initial={reducedMotion ? false : { opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.3, ease: EASE }}
      className="rounded-xl border border-base-300 bg-base-100 p-6 mt-6"
    >
      <h2 className="text-xl font-semibold mb-1">{t('orgSettings.defaults.title')}</h2>
      <p className="text-sm text-base-content/60 mb-4">{t('orgSettings.defaults.description')}</p>

      <div className="space-y-4">
        <div>
          <label htmlFor="org-default-rpm" className="block text-sm mb-1">
            {t('orgSettings.defaults.rateLimitLabel')}
          </label>
          <input
            id="org-default-rpm"
            type="number"
            min="1"
            placeholder="Unlimited"
            disabled={!canEdit || updateDefaults.isPending}
            value={rpm}
            onChange={(e) => setRpm(e.target.value)}
            className={INPUT_CLASS}
          />
          <p className="text-xs text-base-content/50 mt-1">{t('orgSettings.defaults.rateLimitHelp')}</p>
        </div>

        <div>
          <label htmlFor="org-default-budget" className="block text-sm mb-1">
            {t('orgSettings.defaults.budgetLabel')}
          </label>
          <input
            id="org-default-budget"
            type="number"
            min="0"
            step="0.01"
            placeholder="No budget"
            disabled={!canEdit || updateDefaults.isPending}
            value={budget}
            onChange={(e) => setBudget(e.target.value)}
            className={INPUT_CLASS}
          />
          <p className="text-xs text-base-content/50 mt-1">{t('orgSettings.defaults.budgetHelp')}</p>
        </div>
      </div>

      {canEdit && (
        <div className="flex justify-end gap-2 mt-4">
          <Button
            variant="ghost"
            onClick={() => {
              setRpm(data?.default_rate_limit_rpm?.toString() ?? '');
              setBudget(data?.default_budget_monthly_usd?.toString() ?? '');
            }}
            disabled={!dirty || updateDefaults.isPending}
          >
            {t('orgSettings.defaults.cancel')}
          </Button>
          <Button
            onClick={onSave}
            disabled={!canSave}
          >
            {t('orgSettings.defaults.save')}
          </Button>
        </div>
      )}
    </motion.section>
  );
}
