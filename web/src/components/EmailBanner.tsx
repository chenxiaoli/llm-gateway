import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Mail } from 'lucide-react';
import { useAuthStore } from '../stores/authStore';
import { AddEmailModal } from './AddEmailModal';

/**
 * Inline banner prompting pre-Phase-4 users (those whose accounts predate the
 * email-required signup flow) to add an email. Without one they can't receive
 * password-reset emails or email-bound invitations. Hidden once the user adds
 * an email (user.email !== null) or dismisses for the current session.
 *
 * Mounted at the top of Layout's <main>, above the <Outlet />. NOT fixed-
 * positioned — scrolls with the page content. The dismiss flag is per-session
 * and resets on login/logout/register (see authStore).
 */
export function EmailBanner() {
  const { t } = useTranslation();
  const user = useAuthStore((s) => s.user);
  const dismissed = useAuthStore((s) => s.emailBannerDismissed);
  const dismiss = useAuthStore((s) => s.dismissEmailBanner);
  const [modalOpen, setModalOpen] = useState(false);

  if (!user || user.email !== null || dismissed) return null;

  return (
    <>
      <div
        role="status"
        aria-live="polite"
        className="mb-4 flex items-center gap-3 rounded-lg border border-blue-500/30 bg-blue-500/10 px-4 py-2.5 text-sm"
      >
        <Mail className="h-4 w-4 shrink-0 text-blue-500" />
        <span className="flex-1 text-base-content/80">
          {t('emailBanner.message')}
        </span>
        <button
          type="button"
          className="btn btn-sm btn-primary"
          onClick={() => setModalOpen(true)}
        >
          {t('emailBanner.addAction')}
        </button>
        <button
          type="button"
          className="btn btn-sm btn-ghost"
          onClick={dismiss}
          aria-label={t('emailBanner.dismiss')}
        >
          {t('emailBanner.dismiss')}
        </button>
      </div>
      <AddEmailModal open={modalOpen} onClose={() => setModalOpen(false)} />
    </>
  );
}
