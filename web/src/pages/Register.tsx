import { useEffect, useState } from 'react';
import { useNavigate, Link, useSearchParams } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { useAuthStore } from '../stores/authStore';
import { getAuthConfig } from '../api/auth';
import { Button } from '../components/ui/Button';
import { Alert } from '../components/ui/Alert';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';

export default function Register() {
  const { t } = useTranslation();
  const [username, setUsername] = useState('');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [confirm, setConfirm] = useState('');
  const [loading, setLoading] = useState(false);
  const navigate = useNavigate();
  const register = useAuthStore((s) => s.register);
  const setPendingInviteToken = useAuthStore((s) => s.setPendingInviteToken);
  const [params] = useSearchParams();

  // Belt-and-suspenders: if the visitor arrived via /register?invite=... (e.g.
  // forwarded from /accept-invite), make sure the token is in the store so
  // authStore.register() picks it up. /accept-invite already stashes it before
  // navigating here, but this covers the direct-link case.
  const inviteFromUrl = params.get('invite');
  useEffect(() => {
    if (inviteFromUrl) setPendingInviteToken(inviteFromUrl);
  }, [inviteFromUrl, setPendingInviteToken]);

  const { data: authConfig } = useQuery({
    queryKey: ['authConfig'],
    queryFn: getAuthConfig,
    retry: false,
  });

  const registrationDisabled = authConfig !== undefined && !authConfig.allow_registration;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (password !== confirm) {
      toast.error(t('auth.errorMismatch'));
      return;
    }
    if (!username || !password || !email) return;
    setLoading(true);
    try {
      await register({ username, password, email });
      // After register, the user is in "email not verified" limbo. Redirect to
      // /check-email which shows "we sent a verification email to {email}".
      navigate('/check-email', { state: { email } });
    } catch (err: any) {
      const code = err?.response?.data?.error?.code;
      if (code === 'email_in_use') toast.error(t('auth.emailInUse'));
      else if (code === 'email_mismatch') toast.error(t('auth.emailMismatch'));
      else if (code === 'email_required') toast.error(t('auth.emailRequired'));
      else toast.error(getErrorMessage(err, t('auth.errorRegister')));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen flex items-center justify-center bg-base-200">
      <div className="card w-[400px] max-w-[calc(100vw-48px)] bg-base-100 shadow-xl animate-fade-in-up">
        <div className="card-body">
          <div className="flex items-center justify-center gap-3 mb-2">
            <div className="h-11 w-11 rounded-xl bg-primary flex items-center justify-center font-bold text-lg text-primary-content tracking-tight">
              TV
            </div>
            <span className="font-bold text-xl">{t('auth.signUp')}</span>
          </div>

          {registrationDisabled && (
            <Alert variant="warning" className="mb-4">{t('auth.registrationDisabled')}</Alert>
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
                minLength={3}
                disabled={registrationDisabled}
                className="input input-bordered w-full"
              />
            </div>
            <div className="form-control">
              <label className="label"><span className="label-text font-medium">{t('auth.email')}</span></label>
              <input
                type="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                placeholder={t('auth.email')}
                required
                disabled={registrationDisabled}
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
                minLength={6}
                disabled={registrationDisabled}
                className="input input-bordered w-full"
              />
            </div>
            <div className="form-control">
              <label className="label"><span className="label-text font-medium">{t('auth.confirmPassword')}</span></label>
              <input
                type="password"
                value={confirm}
                onChange={(e) => setConfirm(e.target.value)}
                placeholder={t('auth.confirmPassword')}
                required
                disabled={registrationDisabled}
                className="input input-bordered w-full"
              />
            </div>
            <div className="pt-2">
              <Button variant="primary" size="lg" loading={loading} disabled={registrationDisabled} className="w-full">
                {t('auth.register')}
              </Button>
            </div>
          </form>

          <p className="text-center text-sm text-base-content/50 mt-5">
            {t('auth.hasAccount')}{' '}
            <Link to="/login" className="link link-primary">{t('auth.signInLink')}</Link>
          </p>
        </div>
      </div>
    </div>
  );
}
