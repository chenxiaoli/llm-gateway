import { useState } from 'react';
import { Check, Copy } from 'lucide-react';
import { toast } from 'sonner';
import { useTranslation } from 'react-i18next';

interface CopyButtonProps {
  value: string;
  className?: string;
}

export function CopyButton({ value, className }: CopyButtonProps) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);

  const handleCopy = (e: React.MouseEvent) => {
    e.stopPropagation();
    navigator.clipboard.writeText(value);
    setCopied(true);
    toast.success(t('toasts.copiedToClipboard'));
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <button
      type="button"
      onClick={handleCopy}
      className={`btn btn-ghost btn-xs px-1 ${className ?? ''}`}
    >
      {copied
        ? <Check className="h-3.5 w-3.5 text-success" />
        : <Copy className="h-3.5 w-3.5 text-base-content/40" />}
    </button>
  );
}
