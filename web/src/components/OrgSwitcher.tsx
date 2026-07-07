import { useState, useRef, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { ChevronDown, Check, Plus } from 'lucide-react';
import { useAuthStore } from '../stores/authStore';
import { cn } from '../lib/cn';

export function OrgSwitcher() {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const navigate = useNavigate();
  const currentOrg = useAuthStore((s) => s.currentOrg);
  const orgs = useAuthStore((s) => s.orgs);
  const setCurrentOrg = useAuthStore((s) => s.setCurrentOrg);

  useEffect(() => {
    function onClick(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false);
      }
    }
    document.addEventListener('mousedown', onClick);
    return () => document.removeEventListener('mousedown', onClick);
  }, []);

  if (!currentOrg) return null;

  async function switchTo(slug: string) {
    const target = orgs.find((o) => o.slug === slug);
    if (!target) return;
    setOpen(false);
    try {
      await setCurrentOrg(target);
      navigate(`/${slug}/dashboard`);
    } catch {
      // Backend will have surfaced 403; keep current org selected.
      // Toast/log omitted for Plan 2.1 — add proper error UI in Plan 2.2.
    }
  }

  return (
    <div ref={ref} className="relative">
      <button
        type="button"
        onClick={() => setOpen(!open)}
        className="flex w-full items-center justify-between rounded-md px-3 py-2 text-sm hover:bg-white/5"
      >
        <span className="truncate font-medium">{currentOrg.name}</span>
        <ChevronDown className="h-4 w-4 opacity-50" />
      </button>

      {open && (
        <div className="absolute left-0 right-0 top-full z-50 mt-1 rounded-md border border-white/10 bg-zinc-900 py-1 shadow-lg">
          {orgs.map((org) => (
            <button
              key={org.id}
              type="button"
              onClick={() => switchTo(org.slug)}
              className={cn(
                'flex w-full items-center justify-between px-3 py-1.5 text-sm hover:bg-white/5',
                org.slug === currentOrg.slug && 'text-emerald-400',
              )}
            >
              <span className="truncate">{org.name}</span>
              {org.slug === currentOrg.slug && <Check className="h-3 w-3" />}
            </button>
          ))}
          <div className="my-1 border-t border-white/10" />
          <button
            type="button"
            onClick={() => {
              setOpen(false);
              navigate('/orgs/new');
            }}
            className="flex w-full items-center gap-2 px-3 py-1.5 text-sm text-zinc-400 hover:bg-white/5"
          >
            <Plus className="h-4 w-4" /> Create org
          </button>
        </div>
      )}
    </div>
  );
}
