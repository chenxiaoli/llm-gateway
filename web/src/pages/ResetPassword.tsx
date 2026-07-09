import { useEffect, useState } from 'react';
import { Link, useNavigate, useParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';
import { CheckCircle2, XCircle, AlertCircle } from 'lucide-react';
import { confirmPasswordReset, previewPasswordReset } from '../api/auth';
import { LoadingSpinner } from '../components/ui/LoadingSpinner';
import { Button } from '../components/ui/Button';
import { toast } from 'sonner';

const EASE = [0.16, 1, 0.3, 1] as const;

type Status = 'loading' | 'valid' | 'expired' | 'success' | 'error';

/**
 * Landing page for password-reset email links (/reset-password/:token).
 *
 * On mount we GET /auth/password-reset/preview?token=... — this validates the
 * token WITHOUT consuming it, so we know whether to show the new-password
 * form (valid), an expired panel (expired), or a generic error (error). Once
 * the user submits the form we POST /auth/password-reset/confirm, which
 * consumes the token. Backend error codes:
 *   - reset_consumed   → link already used; show inline error on the form
 *   - reset_expired    → flip to the expired panel
 *   - reset_not_found  → treat as expired (don't distinguish — security)
 *   - anything else    → generic inline error
 *
 * On success: toast + flip to the success panel, then auto-redirect to
 * /login after 1500ms.
 */
export default function ResetPassword() {
  const { t } = useTranslation();
  const { token } = useParams<{ token: string }>();
  const navigate = useNavigate();
  const [status, setStatus] = useState<Status>('loading');
  const [password, setPassword] = useState('');
  const [confirm, setConfirm] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [inlineError, setInlineError] = useState<string | null>(null);
  const [retryCount, setRetryCount] = useState(0);

  // Initial preview. Incrementing retryCount from the error panel re-runs this.
  useEffect(() => {
    if (!token) {
      setStatus('expired');
      return;
    }
    let cancelled = false;
    setStatus('loading');
    previewPasswordReset(token)
      .then((p) => {
        if (cancelled) return;
        setStatus(p.valid ? 'valid' : 'expired');
      })
      .catch(() => {
        if (cancelled) return;
        setStatus('error');
      });
    return () => {
      cancelled = true;
    };
  }, [token, retryCount]);

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!token) return;
    if (password !== confirm) {
      toast.error(t('auth.errorMismatch'));
      return;
    }
    if (password.length < 8) {
      toast.error(t('auth.errorShort'));
      return;
    }
    setInlineError(null);
    setSubmitting(true);
    confirmPasswordReset(token, password)
      .then(() => {
        toast.success(t('reset_password.updated'));
        setStatus('success');
        setTimeout(() => navigate('/login'), 1500);
      })
      .catch((err: any) => {
        const code = err?.response?.data?.error?.code;
        if (code === 'reset_consumed') {
          // Token already used — keep the form visible, show inline error.
          setInlineError(t('reset_password.consumed'));
        } else if (code === 'reset_expired' || code === 'reset_not_found') {
          // Don't distinguish not_found from expired — both mean "this link is
          // dead". Flipping to the expired panel is the honest UX.
          setStatus('expired');
        } else {
          setInlineError(t('reset_password.genericError'));
        }
      })
      .finally(() => {
        setSubmitting(false);
      });
  }

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
            <p className="text-sm text-base-content/60">{t('reset_password.loading')}</p>
          </div>
        )}

        {status === 'valid' && (
          <div className="flex flex-col gap-4">
            <h1 className="text-xl font-semibold text-center">{t('reset_password.validTitle')}</h1>
            <form onSubmit={handleSubmit} className="space-y-4">
              <div className="form-control">
                <label className="label"><span className="label-text font-medium">{t('reset_password.passwordLabel')}</span></label>
                <input
                  type="password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  placeholder={t('reset_password.passwordLabel')}
                  required
                  minLength={8}
                  autoFocus
                  className="input input-bordered w-full"
                />
                <p className="text-xs text-base-content/40 mt-1">{t('auth.minChars')}</p>
              </div>
              <div className="form-control">
                <label className="label"><span className="label-text font-medium">{t('reset_password.confirmLabel')}</span></label>
                <input
                  type="password"
                  value={confirm}
                  onChange={(e) => setConfirm(e.target.value)}
                  placeholder={t('reset_password.confirmLabel')}
                  required
                  minLength={8}
                  className="input input-bordered w-full"
                />
              </div>
              {inlineError && (
                <p className="text-sm text-error">{inlineError}</p>
              )}
              <Button type="submit" variant="primary" size="lg" loading={submitting} className="w-full">
                {t('reset_password.submit')}
              </Button>
            </form>
          </div>
        )}

        {status === 'success' && (
          <div className="flex flex-col items-center gap-3 text-center">
            <CheckCircle2 className="h-12 w-12 text-success" />
            <h1 className="text-xl font-semibold">{t('reset_password.updated')}</h1>
            <p className="text-sm text-base-content/60">{t('reset_password.updatedBody')}</p>
          </div>
        )}

        {status === 'expired' && (
          <div className="flex flex-col items-center gap-3 text-center">
            <XCircle className="h-12 w-12 text-error" />
            <h1 className="text-xl font-semibold">{t('reset_password.expiredTitle')}</h1>
            <p className="text-sm text-base-content/60">{t('reset_password.expiredBody')}</p>
            <Link to="/forgot-password" className="btn btn-primary btn-sm mt-2">
              {t('reset_password.requestNew')}
            </Link>
          </div>
        )}

        {status === 'error' && (
          <div className="flex flex-col items-center gap-3 text-center">
            <AlertCircle className="h-12 w-12 text-warning" />
            <h1 className="text-xl font-semibold">{t('reset_password.errorTitle')}</h1>
            <Button
              className="mt-2 w-full"
              onClick={() => {
                // Re-trigger the preview effect.
                setRetryCount((n) => n + 1);
              }}
            >
              {t('reset_password.retry')}
            </Button>
          </div>
        )}
      </motion.div>
    </div>
  );
}
