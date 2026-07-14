import { useTranslation } from 'react-i18next';
import { ShieldAlert } from 'lucide-react';
import { useAuthStore } from '../stores/authStore';
import { displayName } from '../lib/displayName';

/**
 * Top-of-page amber banner shown when a platform_admin is operating in an
 * org via a temp (system-created) membership row. The flag is computed in
 * `/auth/me` and threaded through the auth store. The banner renders null
 * when not impersonating, so mounting it unconditionally is a no-op in the
 * normal case. Layout.tsx shifts the fixed header/main down by 32px while
 * this is visible to avoid overlap.
 */
export function ImpersonationBanner() {
  const impersonating = useAuthStore((s) => s.impersonating);
  const currentOrg = useAuthStore((s) => s.currentOrg);
  const user = useAuthStore((s) => s.user);
  const { t } = useTranslation();

  if (!impersonating || !currentOrg) return null;

  return (
    <div
      role="status"
      aria-live="polite"
      className="fixed top-0 left-0 right-0 z-[60] h-8 flex items-center justify-center gap-2 px-4 text-xs font-medium bg-amber-500/15 border-b border-amber-500/40 text-amber-300"
    >
      <ShieldAlert className="h-3.5 w-3.5 shrink-0" />
      <span className="truncate">
        {t('impersonation.banner', {
          org: currentOrg.name,
          user: user ? displayName(user) : '',
        })}
      </span>
    </div>
  );
}
