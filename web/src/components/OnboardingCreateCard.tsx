import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { apiClient, getErrorMessage } from '../api/client';
import { useAuthStore } from '../stores/authStore';
import { displayName } from '../lib/displayName';
import { Button } from './ui/Button';
import type { AuthResponse } from '../types';

const INPUT_CLASS =
  'w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm text-base-content focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors disabled:opacity-60 disabled:cursor-not-allowed';

/**
 * Slugify a free-form string into the backend's accepted charset
 * (lowercase a-z, 0-9, hyphens). Used to pre-fill the slug from the
 * org name as the user types.
 */
function slugify(s: string): string {
  return s
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9-]+/g, '-')
    .replace(/-+/g, '-')
    .replace(/^-+|-+$/g, '');
}

export function OnboardingCreateCard() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const user = useAuthStore((s) => s.user);
  const applyAuthResponse = useAuthStore((s) => s.applyAuthResponse);

  const [name, setName] = useState(user ? displayName(user) : '');
  const [slug, setSlug] = useState(slugify(user ? displayName(user) : ''));
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  // Keep slug in sync with name unless the user has manually edited it.
  const [slugTouched, setSlugTouched] = useState(false);

  const handleNameChange = (value: string) => {
    setName(value);
    if (!slugTouched) {
      setSlug(slugify(value));
    }
  };

  const handleSlugChange = (value: string) => {
    setSlugTouched(true);
    // Normalize as the user types so they can't paste invalid chars.
    setSlug(slugify(value));
  };

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (busy) return;
    setBusy(true);
    setError(null);
    try {
      const r = await apiClient.post<AuthResponse>('/orgs', {
        name: name.trim(),
        slug: slug.trim(),
      });
      await applyAuthResponse(r.data);
      const targetSlug = useAuthStore.getState().currentOrg?.slug ?? r.data.current_org?.slug;
      if (targetSlug) {
        navigate(`/${targetSlug}/dashboard`, { replace: true });
      }
    } catch (err) {
      const status = (err as { response?: { status?: number } })?.response?.status;
      if (status === 409) {
        setError(t('onboarding.create.errors.slugTaken'));
      } else {
        toast.error(getErrorMessage(err, t('onboarding.create.errors.slugTaken')));
      }
    } finally {
      setBusy(false);
    }
  };

  return (
    <form
      onSubmit={submit}
      className="rounded-2xl border border-base-300/40 bg-base-100 p-6 flex flex-col"
    >
      <h2 className="text-lg font-semibold mb-1">{t('onboarding.create.title')}</h2>
      <p className="text-sm text-base-content/50 mb-5">{t('onboarding.create.subtitle')}</p>

      <div className="space-y-4 flex-1">
        <div>
          <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-1.5 block">
            {t('onboarding.create.name')}
          </label>
          <input
            type="text"
            value={name}
            onChange={(e) => handleNameChange(e.target.value)}
            placeholder={t('onboarding.create.namePlaceholder')}
            disabled={busy}
            autoComplete="off"
            className={INPUT_CLASS}
            required
          />
        </div>
        <div>
          <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-1.5 block">
            {t('onboarding.create.slug')}
          </label>
          <input
            type="text"
            value={slug}
            onChange={(e) => handleSlugChange(e.target.value)}
            placeholder={t('onboarding.create.slugPlaceholder')}
            disabled={busy}
            autoComplete="off"
            spellCheck={false}
            className={`${INPUT_CLASS} font-mono`}
            required
            minLength={3}
            maxLength={64}
          />
          <p className="mt-1.5 text-xs text-base-content/40">
            {t('onboarding.create.slugHint')}
          </p>
        </div>
        {error && (
          <p role="alert" className="text-sm text-error">{error}</p>
        )}
      </div>

      <div className="pt-4">
        <Button type="submit" loading={busy} className="w-full" disabled={!name.trim() || !slug.trim()}>
          {t('onboarding.create.submit')}
        </Button>
      </div>
    </form>
  );
}
