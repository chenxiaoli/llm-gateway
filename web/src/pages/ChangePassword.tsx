import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Lock } from 'lucide-react';
import { motion } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import { changePassword } from '../api/auth';
import { useReducedMotion } from '../hooks/useReducedMotion';
import { Button } from '../components/ui/Button';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';
import { useAuthStore } from '../stores/authStore';

const EASE = [0.16, 1, 0.3, 1] as const;

export default function ChangePassword() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const reducedMotion = useReducedMotion();
  const [currentPw, setCurrentPw] = useState('');
  const [newPw, setNewPw] = useState('');
  const [confirmPw, setConfirmPw] = useState('');
  const [changingPw, setChangingPw] = useState(false);
  const slug = useAuthStore((s) => s.currentOrg?.slug);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (newPw !== confirmPw) {
      toast.error(t('auth.errorMismatch'));
      return;
    }
    if (newPw.length < 8) {
      toast.error(t('auth.errorShort'));
      return;
    }
    setChangingPw(true);
    try {
      await changePassword({ current_password: currentPw, new_password: newPw });
      toast.success(t('auth.successChanged'));
      navigate(slug ? `/${slug}/account` : '/login');
    } catch (err) {
      toast.error(getErrorMessage(err, t('auth.errorChange')));
    } finally {
      setChangingPw(false);
    }
  };

  return (
    <div className="px-6 pb-8">
      <motion.div
        initial={reducedMotion ? false : { opacity: 0, y: -12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={reducedMotion ? { duration: 0 } : { duration: 0.4, ease: EASE }}
        className="mb-8 pt-8"
      >
        <h1 className="text-3xl font-black tracking-tight text-base-content leading-none mb-1">
          {t('auth.changePassword')}
        </h1>
        <p className="text-base text-base-content/50">
          {t('auth.updateAccountPassword')}
        </p>
      </motion.div>

      <motion.div
        initial={reducedMotion ? false : { opacity: 0, y: 12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={reducedMotion ? { duration: 0 } : { duration: 0.35, delay: 0.05, ease: EASE }}
        className="rounded-2xl border border-base-300/40 bg-base-100 p-6 max-w-lg"
      >
        <div className="flex items-center gap-2 mb-5">
          <Lock className="h-5 w-5 text-base-content/40" />
          <h2 className="text-lg font-bold">{t('auth.setNewPassword')}</h2>
        </div>
        <form onSubmit={handleSubmit} className="space-y-4">
          <div className="space-y-1.5">
            <label htmlFor="current-pw" className="text-xs font-semibold uppercase tracking-wider text-base-content/50">
              {t('auth.currentPassword')}
            </label>
            <input
              id="current-pw"
              type="password"
              value={currentPw}
              onChange={(e) => setCurrentPw(e.target.value)}
              required
              className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm text-base-content focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
            />
          </div>
          <div className="space-y-1.5">
            <label htmlFor="new-pw" className="text-xs font-semibold uppercase tracking-wider text-base-content/50">
              {t('auth.newPassword')}
            </label>
            <input
              id="new-pw"
              type="password"
              value={newPw}
              onChange={(e) => setNewPw(e.target.value)}
              required
              minLength={8}
              className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm text-base-content focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
            />
            <p className="text-xs text-base-content/30">{t('auth.minChars')}</p>
          </div>
          <div className="space-y-1.5">
            <label htmlFor="confirm-pw" className="text-xs font-semibold uppercase tracking-wider text-base-content/50">
              {t('auth.confirmNewPassword')}
            </label>
            <input
              id="confirm-pw"
              type="password"
              value={confirmPw}
              onChange={(e) => setConfirmPw(e.target.value)}
              required
              className="w-full h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm text-base-content focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
            />
          </div>
          <div className="flex items-center gap-2 pt-2">
            <Button type="submit" loading={changingPw} className="flex-1">
              {t('auth.updatePassword')}
            </Button>
            <Button type="button" variant="ghost" onClick={() => navigate(slug ? `/${slug}/account` : '/login')}>
              {t('common.cancel')}
            </Button>
          </div>
        </form>
      </motion.div>
    </div>
  );
}
