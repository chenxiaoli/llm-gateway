import { Navigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';
import { useAuthStore, useAuthBootstrap } from '../stores/authStore';
import { getToken } from '../api/client';
import { LoadingSpinner } from '../components/ui/LoadingSpinner';
import { OnboardingCreateCard } from '../components/OnboardingCreateCard';
import { OnboardingJoinCard } from '../components/OnboardingJoinCard';

const EASE = [0.16, 1, 0.3, 1] as const;

/**
 * Inline auth gate for the standalone /onboarding route. The shared
 * RequireAuth in App.tsx renders an <Outlet/> (so it can only be used as a
 * route element inside the org-scoped subtree). /onboarding lives outside
 * that subtree (limbo users have no org), so we inline the same auth check
 * here: loading → spinner, no token → /login, otherwise render the wizard.
 */
function OnboardingGate({ children }: { children: React.ReactNode }) {
  const user = useAuthStore((s) => s.user);
  const { isLoading } = useAuthBootstrap();
  if (isLoading) {
    return (
      <div className="flex h-screen items-center justify-center">
        <LoadingSpinner size="lg" />
      </div>
    );
  }
  if (!user) {
    if (getToken()) {
      return (
        <div className="flex h-screen items-center justify-center">
          <LoadingSpinner size="lg" />
        </div>
      );
    }
    return <Navigate to="/login" replace />;
  }
  return <>{children}</>;
}

export default function Onboarding() {
  const { t } = useTranslation();

  return (
    <OnboardingGate>
      <div className="min-h-screen flex items-center justify-center bg-base-200 px-4 py-10">
        <motion.div
          initial={{ opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.35, ease: EASE }}
          className="max-w-3xl w-full"
        >
          <h1 className="text-2xl font-semibold mb-1">{t('onboarding.title')}</h1>
          <p className="text-sm text-base-content/50 mb-6">{t('onboarding.subtitle')}</p>
          <div className="grid md:grid-cols-2 gap-4 items-stretch">
            <OnboardingCreateCard />
            <OnboardingJoinCard />
          </div>
        </motion.div>
      </div>
    </OnboardingGate>
  );
}
