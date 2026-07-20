import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { motion } from 'framer-motion';
import { toast } from 'sonner';
import { useAuthStore } from '../stores/authStore';
import { useUpdateMyNickname } from '../hooks/useUpdateMyNickname';
import { Button } from '../components/ui/Button';
import { Alert } from '../components/ui/Alert';

const EASE = [0.16, 1, 0.3, 1] as const;

const NICKNAME_MAX = 32;

function validateNickname(raw: string): string | null | Error {
  const trimmed = raw.trim();
  if (trimmed.length === 0) return null; // clear
  if ([...trimmed].length > NICKNAME_MAX) {
    return new Error('too_long');
  }
  // Reject ASCII control chars (0x00-0x1F), DEL (0x7F), C1 control chars
  // (0x80-0x9F), zero-width chars (U+200B-200D, U+FEFF). for...of iterates
  // Unicode code points so emoji are treated as single chars (allowed).
  for (const c of trimmed) {
    const code = c.codePointAt(0)!;
    if (code < 0x20 || code === 0x7f || (code >= 0x80 && code <= 0x9f)) {
      return new Error('control');
    }
    if (code >= 0x200b && code <= 0x200d) return new Error('control');
    if (code === 0xfeff) return new Error('control');
  }
  return trimmed;
}

export default function Profile() {
  const { t } = useTranslation();
  const user = useAuthStore((s) => s.user);
  const updateNickname = useUpdateMyNickname();

  const [input, setInput] = useState(user?.nickname ?? '');
  const [errorKey, setErrorKey] = useState<string | null>(null);

  // Re-sync when the store updates (e.g. after a successful save).
  useEffect(() => {
    setInput(user?.nickname ?? '');
  }, [user?.nickname]);

  const handleSave = () => {
    const validated = validateNickname(input);
    if (validated instanceof Error) {
      setErrorKey(validated.message === 'too_long' ? 'invalidTooLong' : 'invalidControlChars');
      return;
    }
    setErrorKey(null);
    updateNickname.mutate(validated ?? '', {
      onSuccess: (_data, vars) => {
        toast.success(vars.trim().length === 0 ? t('profile.clearedShort') : t('profile.savedShort'));
      },
      onError: () => {
        toast.error(t('profile.invalidControlChars'));
      },
    });
  };

  return (
    <motion.div
      className="px-6 pb-8 max-w-2xl"
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.3, ease: EASE }}
    >
      <h1 className="text-2xl font-semibold mb-6">{t('profile.title')}</h1>

      <section className="space-y-4 rounded-lg border border-base-300 bg-base-100 p-6">
        <div>
          <label className="block text-sm font-medium mb-1">{t('profile.nickname')}</label>
          <input
            type="text"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            placeholder={t('profile.nicknamePlaceholder')}
            maxLength={NICKNAME_MAX * 4} // UTF-8 byte headroom; real check is on chars
            className="w-full rounded-md border border-base-300 bg-base-100 px-3 py-2 text-base"
          />
          <p className="text-xs text-base-content/50 mt-1">{t('profile.nicknameHint')}</p>
        </div>

        {errorKey && (
          <Alert variant="error">{t(`profile.${errorKey}`)}</Alert>
        )}

        <div className="flex justify-end">
          <Button onClick={handleSave} disabled={updateNickname.isPending}>
            {t('profile.save')}
          </Button>
        </div>
      </section>

      <section className="mt-6 space-y-3 rounded-lg border border-base-300 bg-base-100 p-6">
        <div>
          <div className="text-xs text-base-content/50">{t('profile.username')}</div>
          <div className="text-sm">{user?.username || '—'}</div>
        </div>
        <div>
          <div className="text-xs text-base-content/50">{t('profile.email')}</div>
          <div className="text-sm">{user?.email || '—'}</div>
        </div>
      </section>
    </motion.div>
  );
}
