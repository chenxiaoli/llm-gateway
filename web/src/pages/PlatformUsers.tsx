import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { Search } from 'lucide-react';
import {
  listPlatformUsers,
  searchCandidates,
  setPlatformRole,
  type PlatformUserBrief,
} from '../api/admin';

export default function PlatformUsers() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const [query, setQuery] = useState('');

  const adminsQuery = useQuery({
    queryKey: ['platform-users'],
    queryFn: () => listPlatformUsers(),
  });

  const candidatesQuery = useQuery({
    queryKey: ['platform-users-candidates', query],
    queryFn: () => searchCandidates(query),
    enabled: query.length >= 1,
  });

  const grantMutation = useMutation({
    mutationFn: ({ userId, role }: { userId: string; role: 'platform_admin' | null }) =>
      setPlatformRole(userId, role),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['platform-users'] }),
  });

  const admins = adminsQuery.data?.admins ?? [];
  const candidates = candidatesQuery.data ?? [];
  const isLastAdmin = admins.length <= 1;

  return (
    <div className="max-w-4xl">
      <h1 className="text-2xl font-semibold mb-2">{t('platformUsers.title')}</h1>
      <p className="text-sm text-base-content/50 mb-6">{t('platformUsers.subtitle')}</p>

      <section className="mb-10">
        <h2 className="text-sm font-semibold uppercase tracking-[0.12em] text-base-content/40 mb-3">
          {t('platformUsers.currentAdmins')}
        </h2>
        <div className="rounded-lg border border-base-300/60 bg-base-100 overflow-hidden">
          <table className="w-full text-sm">
            <thead className="bg-base-200/50 text-base-content/50 text-xs uppercase">
              <tr>
                <th className="text-left px-4 py-2.5">{t('common.username')}</th>
                <th className="text-left px-4 py-2.5">{t('common.email')}</th>
                <th className="text-right px-4 py-2.5">{t('common.actions')}</th>
              </tr>
            </thead>
            <tbody>
              {admins.map((u) => (
                <tr key={u.id} className="border-t border-base-300/40">
                  <td className="px-4 py-3 font-medium">{u.username}</td>
                  <td className="px-4 py-3 text-base-content/60">{u.email ?? '—'}</td>
                  <td className="px-4 py-3 text-right">
                    {isLastAdmin ? (
                      <span className="text-xs text-base-content/40">—</span>
                    ) : (
                      <button
                        type="button"
                        onClick={() => {
                          if (window.confirm(t('platformUsers.confirmRevoke', { username: u.username }))) {
                            grantMutation.mutate({ userId: u.id, role: null });
                          }
                        }}
                        className="text-xs px-2.5 py-1 rounded-md text-red-500/80 hover:bg-red-500/5"
                      >
                        {t('platformUsers.revoke')}
                      </button>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        {isLastAdmin && (
          <p className="text-xs text-base-content/40 mt-2">{t('platformUsers.lastAdminHint')}</p>
        )}
      </section>

      <section>
        <h2 className="text-sm font-semibold uppercase tracking-[0.12em] text-base-content/40 mb-3">
          {t('platformUsers.addAdmin')}
        </h2>
        <div className="relative mb-3">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-base-content/30" />
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder={t('platformUsers.searchPlaceholder')}
            className="w-full pl-10 pr-4 py-2 rounded-lg border border-base-300/60 bg-base-100 text-sm focus:outline-none focus:border-primary"
          />
        </div>
        {query && (
          <div className="rounded-lg border border-base-300/60 bg-base-100 overflow-hidden">
            {candidates.length === 0 ? (
              <p className="px-4 py-3 text-sm text-base-content/50">{t('platformUsers.noCandidates')}</p>
            ) : (
              <ul>
                {candidates.map((u: PlatformUserBrief) => (
                  <li key={u.id} className="flex items-center justify-between px-4 py-2.5 border-t border-base-300/40 first:border-t-0">
                    <div>
                      <div className="font-medium text-sm">{u.username}</div>
                      <div className="text-xs text-base-content/50">{u.email ?? '—'}</div>
                    </div>
                    <button
                      type="button"
                      onClick={() => grantMutation.mutate({ userId: u.id, role: 'platform_admin' })}
                      className="text-xs px-2.5 py-1 rounded-md bg-primary/10 text-primary hover:bg-primary/15"
                    >
                      {t('platformUsers.grant')}
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </div>
        )}
      </section>

      <p className="text-xs text-base-content/40 mt-8">{t('platformUsers.stalenessNote')}</p>
    </div>
  );
}
