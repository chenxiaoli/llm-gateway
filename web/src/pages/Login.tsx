import { useState } from 'react';
import { useNavigate, Link } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { Eye, EyeOff, AlertCircle } from 'lucide-react';
import { useAuthStore } from '../stores/authStore';
import { getAuthConfig } from '../api/auth';
import { Button } from '../components/ui/Button';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';

export default function Login() {
  const { t } = useTranslation();
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [loading, setLoading] = useState(false);
  const [emailNotVerified, setEmailNotVerified] = useState(false);
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
        // Backend has already dispatched a fresh verification email as part
        // of the email_not_verified response — surface it as a one-click
        // informational panel and let the user head to their inbox.
        setEmailNotVerified(true);
        toast.success(t('auth.verificationSent'));
      } else {
        toast.error(getErrorMessage(err, t('auth.errorInvalid')));
      }
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen flex items-center justify-center bg-base-200">
      <div className="card w-[400px] max-w-[calc(100vw-48px)] bg-base-100 shadow-xl animate-fade-in-up">
        <div className="card-body">
          <h1 className="flex items-center justify-center gap-3 mb-2 font-bold">
            <span className="h-11 w-11 rounded-xl bg-primary flex items-center justify-center font-bold text-lg text-primary-content tracking-tight">
              TV
            </span>
            <span className="text-xl">TokenVis</span>
          </h1>

          {emailNotVerified && (
            <div
              role="alert"
              aria-live="polite"
              className="mb-4 rounded-box border border-warning/40 bg-warning/10 p-4 animate-fade-in"
            >
              <div className="flex items-start gap-2.5">
                <AlertCircle className="mt-0.5 h-4 w-4 shrink-0 text-warning" />
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-medium text-warning">
                    {t('auth.emailNotVerifiedFreshSent')}
                  </p>
                  <button
                    type="button"
                    className="btn btn-ghost btn-xs mt-2 -ml-2 text-base-content/60 hover:text-base-content/80"
                    onClick={() => setEmailNotVerified(false)}
                  >
                    {t('auth.signInLink')}
                  </button>
                </div>
              </div>
            </div>
          )}

          <form onSubmit={handleSubmit} className="space-y-4 mt-4">
            <div className="form-control">
              <label className="label" htmlFor="login-username">
                <span className="label-text font-medium">{t('auth.usernameOrEmail')}</span>
              </label>
              <input
                id="login-username"
                type="text"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                placeholder={t('auth.usernameOrEmail')}
                required
                autoComplete="username"
                className="input input-bordered w-full"
              />
            </div>
            <div className="form-control">
              <label className="label" htmlFor="login-password">
                <span className="label-text font-medium">{t('auth.password')}</span>
              </label>
              <div className="relative">
                <input
                  id="login-password"
                  type={showPassword ? 'text' : 'password'}
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  placeholder={t('auth.password')}
                  required
                  autoComplete="current-password"
                  className="input input-bordered w-full pr-10"
                />
                <button
                  type="button"
                  onClick={() => setShowPassword((s) => !s)}
                  aria-label={showPassword ? 'Hide password' : 'Show password'}
                  className="absolute right-1 top-1/2 -translate-y-1/2 h-9 w-9 flex items-center justify-center rounded-md text-base-content/40 hover:text-base-content/70 hover:bg-base-200 transition-colors"
                >
                  {showPassword ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                </button>
              </div>
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
