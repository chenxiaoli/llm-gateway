import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { Modal } from './ui/Modal';
import { Button } from './ui/Button';
import { setMyEmail } from '../api/auth';
import { useAuthStore } from '../stores/authStore';
import { getErrorMessage } from '../api/client';

/**
 * Modal form for EmailBanner's "Add email" action. On success the response's
 * fresh user fields are pushed into the authStore so the banner disappears
 * immediately. The backend dispatches a verification email as a side effect
 * — the user clicks through from their inbox separately.
 *
 * Error mapping: 409 `email_in_use` is the only typed variant the backend
 * returns here; malformed input surfaces as 400 BadRequest with no `code`
 * field (validate_email's string error is wrapped in ApiError::BadRequest),
 * so it falls through to the generic-error branch via getErrorMessage.
 */
export function AddEmailModal({ open, onClose }: { open: boolean; onClose: () => void }) {
  const { t } = useTranslation();
  const setUser = useAuthStore((s) => s.setUser);
  const currentUser = useAuthStore((s) => s.user);
  const [email, setEmail] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  function handleClose() {
    setEmail('');
    setError(null);
    onClose();
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!email || !currentUser) return;
    setSaving(true);
    setError(null);
    try {
      const me = await setMyEmail(email);
      // Thread the fresh user fields back into the store so EmailBanner
      // disappears immediately.
      setUser({
        id: me.id,
        username: me.username,
        platform_role: me.platform_role,
        email: me.email,
        email_verified_at: me.email_verified_at,
      });
      toast.success(t('addEmailModal.success'));
      handleClose();
    } catch (err: any) {
      const code = err?.response?.data?.error?.code;
      if (code === 'email_in_use') {
        setError(t('addEmailModal.errorInUse'));
      } else {
        setError(getErrorMessage(err, t('addEmailModal.errorGeneric')));
      }
    } finally {
      setSaving(false);
    }
  }

  return (
    <Modal
      open={open}
      onClose={handleClose}
      title={t('addEmailModal.title')}
      width={440}
      footer={
        <>
          <Button variant="ghost" onClick={handleClose} disabled={saving}>
            {t('addEmailModal.cancel')}
          </Button>
          <Button variant="primary" onClick={handleSubmit} loading={saving} disabled={!email}>
            {t('addEmailModal.submit')}
          </Button>
        </>
      }
    >
      <form onSubmit={handleSubmit} className="space-y-3">
        <p className="text-sm text-base-content/60">
          {t('addEmailModal.body')}
        </p>
        <div className="form-control">
          <label className="label" htmlFor="add-email-input">
            <span className="label-text font-medium">{t('addEmailModal.emailLabel')}</span>
          </label>
          <input
            id="add-email-input"
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            placeholder={t('addEmailModal.emailPlaceholder')}
            required
            autoFocus
            autoComplete="email"
            className="input input-bordered w-full"
          />
        </div>
        {error && <p className="text-sm text-error">{error}</p>}
      </form>
    </Modal>
  );
}
