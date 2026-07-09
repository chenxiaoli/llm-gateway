import { useEffect, useState } from 'react';
import { Link, useNavigate, useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';
import { toast } from 'sonner';
import { previewInvitation, acceptInvitation } from '../api/invitations';
import { getErrorMessage } from '../api/client';
import { useAuthStore, useAuthBootstrap } from '../stores/authStore';
import { LoadingSpinner } from '../components/ui/LoadingSpinner';
import { Button } from '../components/ui/Button';
import type { InvitationPreview } from '../types';

const EASE = [0.16, 1, 0.3, 1] as const;

type Status = 'loading' | 'ok' | 'gone' | 'error';

/**
 * Public landing page for invitation links (/accept-invite?token=...).
 *
 * The page is reachable whether or not the visitor is signed in, so the UI
 * branches on the auth store state rather than being gated by a route guard:
 *
 *   logged out           → preview + "Sign up to accept" / "Log in" buttons
 *   logged in, member    → "you're already a member" + "Go to {{org}}" link
 *   logged in, not member→ preview + Accept / Decline buttons
 *
 * The token lives in the URL query string, so a global Referrer-Policy meta
 * tag in index.html (no-referrer) prevents it from leaking via the Referer
 * header on outbound navigation.
 */
export default function AcceptInvite() {
  const { t } = useTranslation();
  const [params] = useSearchParams();
  const navigate = useNavigate();
  const token = params.get('token') ?? '';

  const user = useAuthStore((s) => s.user);
  const orgs = useAuthStore((s) => s.orgs);
  const applyAuthResponse = useAuthStore((s) => s.applyAuthResponse);
  const setPendingInviteToken = useAuthStore((s) => s.setPendingInviteToken);
  const { isLoading } = useAuthBootstrap();

  const [preview, setPreview] = useState<InvitationPreview | null>(null);
  const [status, setStatus] = useState<Status>('loading');
  const [accepting, setAccepting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!token) {
      setStatus('gone');
      return;
    }
    let cancelled = false;
    previewInvitation(token)
      .then((p) => {
        if (cancelled) return;
        setPreview(p);
        setStatus('ok');
      })
      .catch((err) => {
        if (cancelled) return;
        // 410 Gone (expired/revoked/already-used) and other preview errors
        // both render the "gone" UI — there's no useful preview to show.
        const code = (err as { response?: { status?: number } })?.response?.status;
        if (code === 410 || code === 404 || code === 400) {
          setStatus('gone');
        } else {
          // Network/unknown error: distinct from gone — the link itself may be
          // fine, the server just couldn't be reached.
          setStatus('error');
        }
      });
    return () => {
      cancelled = true;
    };
  }, [token]);

  if (isLoading) {
    return (
      <div className="flex h-screen items-center justify-center">
        <LoadingSpinner size="lg" />
      </div>
    );
  }

  if (status === 'loading') {
    return (
      <div className="min-h-screen flex items-center justify-center bg-base-200">
        <LoadingSpinner size="lg" />
      </div>
    );
  }

  if (status === 'error') {
    return (
      <div className="min-h-screen flex items-center justify-center bg-base-200 px-4 py-10">
        <motion.div
          initial={{ opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.35, ease: EASE }}
          className="max-w-md w-full p-8 card bg-base-100 shadow-xl"
        >
          <h1 className="text-xl font-semibold mb-2">{t('acceptInvite.error.title')}</h1>
          <p className="text-sm text-base-content/60 mb-6">{t('acceptInvite.error.description')}</p>
          <Link to="/login" className="btn btn-ghost btn-sm">{t('acceptInvite.gone.back')}</Link>
        </motion.div>
      </div>
    );
  }

  if (status === 'gone' || !preview) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-base-200 px-4 py-10">
        <motion.div
          initial={{ opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.35, ease: EASE }}
          className="max-w-md w-full p-8 card bg-base-100 shadow-xl"
        >
          <h1 className="text-xl font-semibold mb-2">{t('acceptInvite.gone.title')}</h1>
          <p className="text-sm text-base-content/60 mb-6">{t('acceptInvite.gone.description')}</p>
          <Link to="/login" className="btn btn-ghost btn-sm">{t('acceptInvite.gone.back')}</Link>
        </motion.div>
      </div>
    );
  }

  const alreadyMember = orgs.some((o) => o.slug === preview.org_slug);
  // Limbo users (zero orgs) clicking Decline would otherwise bounce back to
  // /accept-invite via OnboardingRedirect if sent to `/`. Send them to
  // /onboarding instead, acknowledging they still need to set up a workspace.
  const declineHref = orgs.length === 0 ? '/onboarding' : '/';

  async function handleAccept() {
    if (accepting) return;
    setAccepting(true);
    setError(null);
    try {
      const resp = await acceptInvitation({ token });
      await applyAuthResponse(resp);
      const slug = useAuthStore.getState().currentOrg?.slug ?? resp.current_org?.slug;
      if (slug) {
        navigate(`/${slug}/dashboard`, { replace: true });
      }
    } catch (err) {
      const code = (err as { response?: { status?: number } })?.response?.status;
      if (code === 410) {
        setError(t('acceptInvite.errors.invalidToken'));
      } else if (code === 409) {
        setError(t('acceptInvite.errors.alreadyAccepted'));
      } else {
        toast.error(getErrorMessage(err, t('acceptInvite.errors.invalidToken')));
      }
    } finally {
      setAccepting(false);
    }
  }

  function handleSignUp() {
    // Stash the token in the store so the register() call picks it up after
    // the user completes the sign-up form. The URL query param is for UX
    // continuity (visible in the address bar) — the actual consumption is via
    // the store in authStore.register().
    setPendingInviteToken(token);
    navigate(`/register?invite=${encodeURIComponent(token)}`);
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-base-200 px-4 py-10">
      <motion.div
        initial={{ opacity: 0, y: 12 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.35, ease: EASE }}
        className="max-w-md w-full p-8 card bg-base-100 shadow-xl"
      >
        <h1 className="text-xl font-semibold mb-1">{t('acceptInvite.title', { org: preview.org_name })}</h1>
        <p className="text-sm text-base-content/60 mb-1">{t('acceptInvite.inviter', { user: preview.inviter_username })}</p>
        <p className="text-sm text-base-content/60 mb-6">{t('acceptInvite.role', { role: preview.role })}</p>

        {error && (
          <p role="alert" className="text-sm text-error mb-4">{error}</p>
        )}

        {alreadyMember ? (
          <div className="space-y-3">
            <p className="text-sm text-base-content/70">{t('acceptInvite.alreadyMember', { org: preview.org_name })}</p>
            <Link to={`/${preview.org_slug}/dashboard`} className="btn btn-primary btn-block">
              {t('acceptInvite.goToOrg', { org: preview.org_name })}
            </Link>
          </div>
        ) : user ? (
          <div className="flex gap-2">
            <Button onClick={handleAccept} loading={accepting} className="flex-1">
              {t('acceptInvite.accept')}
            </Button>
            <Link to={declineHref} className="btn btn-ghost flex-1">{t('acceptInvite.decline')}</Link>
          </div>
        ) : (
          <div className="flex gap-2">
            <Button onClick={handleSignUp} className="flex-1">
              {t('acceptInvite.signUp')}
            </Button>
            <Link
              to={`/login?next=${encodeURIComponent(`/accept-invite?token=${token}`)}`}
              className="btn btn-ghost flex-1"
            >
              {t('acceptInvite.logIn')}
            </Link>
          </div>
        )}
      </motion.div>
    </div>
  );
}
