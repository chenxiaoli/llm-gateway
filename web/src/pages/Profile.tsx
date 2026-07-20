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

// Mirrors backend `validate_nickname` in crates/api/src/auth.rs.
// Returns the trimmed nickname to save, `null` to clear, or an error key.
function validateNickname(raw: string): string | null | 'too_long' | 'invalid' {
  const trimmed = raw.trim();
  if (trimmed.length === 0) return null; // clear
  // Array.from iterates by Unicode code point (matches Rust's chars()).
  const chars = Array.from(trimmed);
  if (chars.length > NICKNAME_MAX) return 'too_long';
  for (const ch of chars) {
    const code = ch.codePointAt(0)!;
    // C0 controls (U+00-U+1F) + DEL (U+7F) + C1 controls (U+80-U+9F).
    if (code < 0x20 || code === 0x7f || (code >= 0x80 && code <= 0x9f)) return 'invalid';
    // Mirrors backend's explicit zero-width ranges.
    if (code >= 0x200b && code <= 0x200d) return 'invalid';
    if (code === 0xfeff) return 'invalid';
    // Mirrors backend's `char::is_control()` which covers Unicode categories
    // Cc and Cf. C0/C1 are handled above; reject the remaining Cf chars the
    // backend would reject: SHY, bidirectional marks, bidi overrides (a real
    // spoofing risk — U+202E can flip "admin" to "nimda"), WJ, and the
    // deprecated U+2060-U+206F format range.
    if (code === 0x00ad) return 'invalid';
    if (code === 0x200e || code === 0x200f) return 'invalid';
    if (code >= 0x202a && code <= 0x202e) return 'invalid';
    if (code >= 0x2060 && code <= 0x206f) return 'invalid';
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
    if (validated === 'too_long') {
      setErrorKey('invalidTooLong');
      return;
    }
    if (validated === 'invalid') {
      setErrorKey('invalidControlChars');
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
