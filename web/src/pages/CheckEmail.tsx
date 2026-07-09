import { useState } from 'react';
import { Link, useLocation, Navigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';
import { MailCheck } from 'lucide-react';
import { resendVerification } from '../api/auth';
import { Button } from '../components/ui/Button';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';

const EASE = [0.16, 1, 0.3, 1] as const;

interface CheckEmailLocationState {
  email?: string;
}

/**
 * Post-registration landing page telling the user "we sent you a verification
 * email — click the link inside". Reached from Register.tsx via
 * `navigate('/check-email', { state: { email } })`. If a visitor lands here
 * with no router state (e.g. direct hit / refresh), bounce them to /login —
 * we have no email to show and the registration intent is gone.
 */
export default function CheckEmail() {
  const { t } = useTranslation();
  const location = useLocation();
  const state = (location.state ?? {}) as CheckEmailLocationState;
  const email = state.email;
  const [resending, setResending] = useState(false);

  if (!email) {
    return <Navigate to="/login" replace />;
  }

  async function handleResend() {
    if (!email) return;
    setResending(true);
    try {
      await resendVerification(email);
      toast.success(t('check_email.resent'));
    } catch (err) {
      // The backend degrades resend errors to a warn log; an unexpected error
      // here is genuinely a server problem, not a user-fixable one.
      toast.error(getErrorMessage(err, t('check_email.resent')));
    } finally {
      setResending(false);
    }
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-base-200 px-4 py-10">
      <motion.div
        initial={{ opacity: 0, y: 12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.35, ease: EASE }}
        className="max-w-md w-full p-8 card bg-base-100 shadow-xl text-center"
      >
        <div className="flex justify-center mb-4">
          <MailCheck className="h-12 w-12 text-primary" />
        </div>
        <h1 className="text-xl font-semibold mb-2">{t('check_email.title')}</h1>
        <p className="text-sm text-base-content/60 mb-6">
          {t('check_email.body', { email })}
        </p>
        <div className="flex flex-col gap-2">
          <Button variant="ghost" onClick={handleResend} loading={resending}>
            {t('check_email.resend')}
          </Button>
          <Link to="/login" className="btn btn-ghost btn-sm">{t('check_email.go_to_login')}</Link>
        </div>
      </motion.div>
    </div>
  );
}
