import { useState } from 'react';
import { useNavigate, Link } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { useAuthStore } from '../stores/authStore';
import { getAuthConfig, resendVerification } from '../api/auth';
import { Button } from '../components/ui/Button';
import { Alert } from '../components/ui/Alert';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';

export default function Login() {
  const { t } = useTranslation();
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [loading, setLoading] = useState(false);
  const [emailNotVerified, setEmailNotVerified] = useState(false);
  const [resendEmail, setResendEmail] = useState('');
  const [resending, setResending] = useState(false);
  const navigate = useNavigate();
  const login = useAuthStore((s) => s.login);

  const { data: authConfig } = useQuery({
    queryKey: ['authConfig'],
    queryFn: getAuthConfig,
    retry: false,
  });

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!username || !password) return;
    setLoading(true);
    try {
      await login({ username, password });
      const slug = useAuthStore.getState().currentOrg?.slug;
      navigate(slug ? `/${slug}/dashboard` : '/login');
    } catch (err: any) {
      const code = err?.response?.data?.error?.code;
      if (code === 'email_not_verified') {
        // Backend rejected the login because the user hasn't verified their
        // email yet. Surface an inline resend panel instead of a toast so the
        // user has an obvious next step. The user types the email here (it
        // may be different from the username's email if they have access to
        // a different inbox).
        setEmailNotVerified(true);
      } else {
        toast.error(getErrorMessage(err, t('auth.errorInvalid')));
      }
    } finally {
      setLoading(false);
    }
  };

  async function handleResend() {
    if (!resendEmail) return;
    setResending(true);
    try {
      await resendVerification(resendEmail);
      toast.success(t('auth.verificationSent'));
    } catch (err) {
      // The backend degrades resend errors to a warn log (doesn't leak
      // whether the email exists), so a failure here is genuinely
      // unexpected — show the generic error toast.
      toast.error(getErrorMessage(err, t('auth.errorInvalid')));
    } finally {
      setResending(false);
    }
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-base-200">
      <div className="card w-[400px] max-w-[calc(100vw-48px)] bg-base-100 shadow-xl animate-fade-in-up">
        <div className="card-body">
          <div className="flex items-center justify-center gap-3 mb-2">
            <div className="h-11 w-11 rounded-xl bg-primary flex items-center justify-center font-bold text-lg text-primary-content tracking-tight">
              TV
            </div>
            <span className="font-bold text-xl">TokenVis</span>
          </div>

          {emailNotVerified && (
            <Alert variant="warning" className="mb-4">
              <div className="flex flex-col gap-2 w-full">
                <span>{t('auth.emailNotVerifiedMessage')}</span>
                <div className="flex gap-2 mt-1">
                  <input
                    type="email"
                    value={resendEmail}
                    onChange={(e) => setResendEmail(e.target.value)}
                    placeholder={t('auth.email')}
                    className="input input-bordered input-sm flex-1"
                  />
                  <Button size="sm" onClick={handleResend} loading={resending} disabled={!resendEmail}>
                    {t('auth.resendVerification')}
                  </Button>
                </div>
                <button
                  type="button"
                  className="btn btn-ghost btn-xs self-start mt-1"
                  onClick={() => {
                    setEmailNotVerified(false);
                    setResendEmail('');
                  }}
                >
                  {t('auth.signIn')}
                </button>
              </div>
            </Alert>
          )}

          <form onSubmit={handleSubmit} className="space-y-4 mt-4">
            <div className="form-control">
              <label className="label"><span className="label-text font-medium">{t('auth.username')}</span></label>
              <input
                type="text"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                placeholder={t('auth.username')}
                required
                className="input input-bordered w-full"
              />
            </div>
            <div className="form-control">
              <label className="label"><span className="label-text font-medium">{t('auth.password')}</span></label>
              <input
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                placeholder={t('auth.password')}
                required
                className="input input-bordered w-full"
              />
            </div>
            <div className="text-right -mt-2">
              <Link to="/forgot-password" className="link link-primary text-sm">
                {t('auth.forgotPassword')}
              </Link>
            </div>
            <div className={authConfig?.allow_registration ? 'pt-2' : ''}>
              <Button variant="primary" size="lg" loading={loading} className="w-full">
                {t('auth.signIn')}
              </Button>
            </div>
          </form>

          {authConfig?.allow_registration && (
            <p className="text-center text-sm text-base-content/50 mt-5">
              {t('auth.noAccount')}{' '}
              <Link to="/register" className="link link-primary">{t('auth.createOne')}</Link>
            </p>
          )}
        </div>
      </div>
    </div>
  );
}
