import { Navigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';
import { useAuthGate } from '../stores/authStore';
import { LoadingSpinner } from '../components/ui/LoadingSpinner';
import { OnboardingCreateCard } from '../components/OnboardingCreateCard';

const EASE = [0.16, 1, 0.3, 1] as const;

/**
 * Inline auth gate — mirrors /onboarding's OnboardingGate. The route lives
 * outside the org-scoped subtree (URL has no org slug) and outside the
 * shared RequireAuth wrapper, so the page runs its own gate: loading →
 * spinner, no token → /login, otherwise render the form.
 *
 * A limbo user (zero org memberships) hitting /orgs/new is bounced to
 * /onboarding by the global OnboardingRedirect in App.tsx before this
 * component mounts, so we don't need a special case here.
 */
function OrgCreateGate({ children }: { children: React.ReactNode }) {
  const status = useAuthGate();
  if (status === 'loading') {
    return (
      <div className="flex h-screen items-center justify-center">
        <LoadingSpinner size="lg" />
      </div>
    );
  }
  if (status === 'login') {
    return <Navigate to="/login" replace />;
  }
  return <>{children}</>;
}

export default function OrgCreate() {
  const { t } = useTranslation();
  return (
    <OrgCreateGate>
      <div className="min-h-screen flex items-center justify-center bg-base-200 px-4 py-10">
        <motion.div
          initial={{ opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.35, ease: EASE }}
          className="max-w-md w-full"
        >
          <h1 className="text-2xl font-semibold mb-1">{t('orgCreate.title')}</h1>
          <p className="text-sm text-base-content/50 mb-6">{t('orgCreate.subtitle')}</p>
          <OnboardingCreateCard />
        </motion.div>
      </div>
    </OrgCreateGate>
  );
}
