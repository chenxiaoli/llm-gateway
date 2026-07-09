import { useState } from 'react';
import { Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';
import { MailCheck } from 'lucide-react';
import { requestPasswordReset } from '../api/auth';
import { Button } from '../components/ui/Button';

const EASE = [0.16, 1, 0.3, 1] as const;

/**
 * Forgot-password flow entry point. The user types their email, we POST to
 * /auth/password-reset/request, and — regardless of whether the request
 * succeeded or the email even exists — we show the same "if an account
 * exists, we've sent a link" panel. The backend is intentionally always-204
 * (does not leak whether the email is registered), so this is the only honest
 * message we can show.
 */
export default function ForgotPassword() {
  const { t } = useTranslation();
  const [email, setEmail] = useState('');
  const [loading, setLoading] = useState(false);
  const [sent, setSent] = useState(false);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!email) return;
    setLoading(true);
    try {
      await requestPasswordReset(email);
    } catch {
      // Backend is always-204 by design. If we somehow get an error, swallow
      // it — surfacing a distinct error message would leak whether the email
      // is registered, defeating the always-204 guarantee.
    } finally {
      setLoading(false);
      setSent(true);
    }
  }

  if (sent) {
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
          <h1 className="text-xl font-semibold mb-2">{t('forgot_password.sentTitle')}</h1>
          <p className="text-sm text-base-content/60 mb-6">
            {t('forgot_password.sentBody', { email })}
          </p>
          <Link to="/login" className="btn btn-ghost btn-sm">{t('forgot_password.backToLogin')}</Link>
        </motion.div>
      </div>
    );
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-base-200">
      <motion.div
        initial={{ opacity: 0, y: 12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.35, ease: EASE }}
        className="card w-[400px] max-w-[calc(100vw-48px)] bg-base-100 shadow-xl"
      >
        <div className="card-body">
          <div className="flex items-center justify-center gap-3 mb-2">
            <div className="h-11 w-11 rounded-xl bg-primary flex items-center justify-center font-bold text-lg text-primary-content tracking-tight">
              TV
            </div>
            <span className="font-bold text-xl">TokenVis</span>
          </div>

          <h2 className="text-center text-lg font-semibold mt-2">{t('forgot_password.title')}</h2>

          <form onSubmit={handleSubmit} className="space-y-4 mt-4">
            <div className="form-control">
              <label className="label"><span className="label-text font-medium">{t('forgot_password.emailLabel')}</span></label>
              <input
                type="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                placeholder={t('forgot_password.emailPlaceholder')}
                required
                autoFocus
                autoComplete="email"
                className="input input-bordered w-full"
              />
            </div>
            <div className="pt-2">
              <Button type="submit" variant="primary" size="lg" loading={loading} className="w-full">
                {t('forgot_password.submit')}
              </Button>
            </div>
          </form>

          <p className="text-center text-sm text-base-content/50 mt-5">
            <Link to="/login" className="link link-primary">{t('forgot_password.backToLogin')}</Link>
          </p>
        </div>
      </motion.div>
    </div>
  );
}
