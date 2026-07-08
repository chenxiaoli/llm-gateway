import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Check, Copy } from 'lucide-react';
import { cn } from '../lib/cn';

interface CopyableInviteLinkProps {
  url: string;
  /** ISO timestamp. When provided, an "expires in N days" hint is rendered. */
  expiresAt?: string;
  className?: string;
}

/**
 * Truncated invite URL + copy button + "expires in N days" hint.
 *
 * Only renders the expiry hint when `expiresAt` is supplied — callers should
 * omit it for already-accepted / revoked invitations where the hint would be
 * irrelevant.
 */
export function CopyableInviteLink({ url, expiresAt, className }: CopyableInviteLinkProps) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);

  async function copy() {
    try {
      await navigator.clipboard.writeText(url);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // navigator.clipboard may be unavailable (non-HTTPS / jsdom) — silently
      // no-op so the button doesn't throw in tests.
    }
  }

  let expiryHint: string | null = null;
  if (expiresAt) {
    const msLeft = new Date(expiresAt).getTime() - Date.now();
    const daysLeft = Math.ceil(msLeft / 86400000);
    if (msLeft <= 0) {
      expiryHint = t('invitations.expired');
    } else if (daysLeft === 1) {
      expiryHint = t('invitations.expiresInOneDay');
    } else {
      expiryHint = t('invitations.expiresInDays', { count: daysLeft });
    }
  }

  return (
    <div className={cn('flex items-center gap-2 min-w-0', className)}>
      <code className="text-xs bg-base-200/60 px-2 py-1 rounded truncate max-w-[18rem] block">
        {url}
      </code>
      <button
        type="button"
        onClick={copy}
        className="btn btn-ghost btn-xs px-1"
        aria-label={t('common.copy')}
      >
        {copied
          ? <Check className="h-3.5 w-3.5 text-success" />
          : <Copy className="h-3.5 w-3.5 text-base-content/40" />}
      </button>
      {expiryHint && (
        <span className="text-xs text-base-content/50 whitespace-nowrap">{expiryHint}</span>
      )}
    </div>
  );
}
