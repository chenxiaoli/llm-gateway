import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { acceptInvitation } from '../api/invitations';
import { getErrorMessage } from '../api/client';
import { useAuthStore } from '../stores/authStore';
import { Button } from './ui/Button';

const INPUT_CLASS =
  'w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm text-base-content focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors disabled:opacity-60 disabled:cursor-not-allowed';

/**
 * Accept either a bare invitation token or a full accept-invite URL
 * (e.g. https://app.example.com/accept-invite?token=abc) and return
 * just the token portion. Falls back to the raw string if it isn't a
 * parseable URL.
 */
function extractToken(raw: string): string {
  const trimmed = raw.trim();
  if (!trimmed) return trimmed;
  try {
    const u = new URL(trimmed);
    const tok = u.searchParams.get('token');
    return tok ?? trimmed;
  } catch {
    return trimmed;
  }
}

export function OnboardingJoinCard() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const applyAuthResponse = useAuthStore((s) => s.applyAuthResponse);

  const [input, setInput] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (busy) return;
    const token = extractToken(input);
    if (!token) return;
    setBusy(true);
    setError(null);
    try {
      const resp = await acceptInvitation({ token });
      await applyAuthResponse(resp);
      const targetSlug = useAuthStore.getState().currentOrg?.slug ?? resp.current_org?.slug;
      if (targetSlug) {
        navigate(`/${targetSlug}/dashboard`, { replace: true });
      }
    } catch (err) {
      const status = (err as { response?: { status?: number } })?.response?.status;
      // 410 Gone (expired/revoked) and 409 Conflict (race-loser: invitation
      // already accepted) both surface as "no longer valid" to the user.
      if (status === 410 || status === 409) {
        setError(t('onboarding.join.errors.invalidToken'));
      } else {
        toast.error(getErrorMessage(err, t('onboarding.join.errors.invalidToken')));
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
      <h2 className="text-lg font-semibold mb-1">{t('onboarding.join.title')}</h2>
      <p className="text-sm text-base-content/50 mb-5">{t('onboarding.join.subtitle')}</p>

      <div className="space-y-4 flex-1">
        <div>
          <label className="text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-1.5 block">
            {t('onboarding.join.tokenLabel')}
          </label>
          <input
            type="text"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            placeholder={t('onboarding.join.tokenPlaceholder')}
            disabled={busy}
            autoComplete="off"
            spellCheck={false}
            className={`${INPUT_CLASS} font-mono`}
            required
          />
        </div>
        {error && (
          <p role="alert" className="text-sm text-error">{error}</p>
        )}
      </div>

      <div className="pt-4">
        <Button type="submit" loading={busy} className="w-full" disabled={!input.trim()}>
          {t('onboarding.join.submit')}
        </Button>
      </div>
    </form>
  );
}
