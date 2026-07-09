import { useEffect, useState } from 'react';
import { Link, useNavigate, useParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';
import { CheckCircle2, XCircle, AlertCircle } from 'lucide-react';
import { verifyEmail } from '../api/auth';
import { LoadingSpinner } from '../components/ui/LoadingSpinner';
import { Button } from '../components/ui/Button';

const EASE = [0.16, 1, 0.3, 1] as const;

type Status = 'loading' | 'ok' | 'expired' | 'error';

/**
 * Landing page for email verification links (/verify-email/:token).
 *
 * On mount the token from the URL is POSTed to /auth/verify-email. The backend
 * either marks the user verified (204 → ok), reports the token expired/invalid
 * (410 with verification_expired → expired), or hits an unexpected error
 * (anything else → error). Each branch renders a distinct panel.
 */
export default function VerifyEmail() {
  const { t } = useTranslation();
  const { token } = useParams<{ token: string }>();
  const navigate = useNavigate();
  const [status, setStatus] = useState<Status>('loading');

  useEffect(() => {
    if (!token) {
      setStatus('expired');
      return;
    }
    let cancelled = false;
    verifyEmail(token)
      .then(() => {
        if (!cancelled) setStatus('ok');
      })
      .catch((err: any) => {
        if (cancelled) return;
        const code = err?.response?.data?.error?.code;
        // Backend returns verification_expired on 410. Some legacy links may
        // also surface as 404 verification_not_found — treat both as expired.
        if (code === 'verification_expired' || code === 'verification_not_found') {
          setStatus('expired');
        } else {
          setStatus('error');
        }
      });
    return () => {
      cancelled = true;
    };
  }, [token]);

  return (
    <div className="min-h-screen flex items-center justify-center bg-base-200 px-4 py-10">
      <motion.div
        initial={{ opacity: 0, y: 12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.35, ease: EASE }}
        className="max-w-md w-full p-8 card bg-base-100 shadow-xl"
      >
        {status === 'loading' && (
          <div className="flex flex-col items-center gap-3 py-6">
            <LoadingSpinner size="lg" />
            <p className="text-sm text-base-content/60">{t('verify_email.verifying')}</p>
          </div>
        )}

        {status === 'ok' && (
          <div className="flex flex-col items-center gap-3 text-center">
            <CheckCircle2 className="h-12 w-12 text-success" />
            <h1 className="text-xl font-semibold">{t('verify_email.verified_title')}</h1>
            <p className="text-sm text-base-content/60">{t('verify_email.verified_body')}</p>
            <Button className="mt-2 w-full" onClick={() => navigate('/login')}>
              {t('verify_email.continue_to_login')}
            </Button>
          </div>
        )}

        {status === 'expired' && (
          <div className="flex flex-col items-center gap-3 text-center">
            <XCircle className="h-12 w-12 text-error" />
            <h1 className="text-xl font-semibold">{t('verify_email.expired_title')}</h1>
            <p className="text-sm text-base-content/60">{t('verify_email.expired_body')}</p>
            <Link to="/login" className="btn btn-ghost btn-sm mt-2">{t('verify_email.continue_to_login')}</Link>
          </div>
        )}

        {status === 'error' && (
          <div className="flex flex-col items-center gap-3 text-center">
            <AlertCircle className="h-12 w-12 text-warning" />
            <h1 className="text-xl font-semibold">{t('verify_email.error_title')}</h1>
            <Button
              className="mt-2 w-full"
              onClick={() => {
                // Re-trigger the effect by flipping back through loading.
                setStatus('loading');
                // Defer the retry to the next tick so the effect re-runs.
                setTimeout(() => {
                  if (token) {
                    verifyEmail(token)
                      .then(() => setStatus('ok'))
                      .catch((err: any) => {
                        const code = err?.response?.data?.error?.code;
                        if (code === 'verification_expired' || code === 'verification_not_found') {
                          setStatus('expired');
                        } else {
                          setStatus('error');
                        }
                      });
                  } else {
                    setStatus('expired');
                  }
                }, 0);
              }}
            >
              {t('verify_email.retry')}
            </Button>
          </div>
        )}
      </motion.div>
    </div>
  );
}
