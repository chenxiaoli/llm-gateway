import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import i18n from '../i18n';
import { useAuthStore } from '../stores/authStore';
import { getOrgDefaults, updateOrgDefaults, type OrgDefaults } from '../api/orgs';
import { getErrorMessage } from '../api/client';

export function useGetOrgDefaults() {
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useQuery({
    queryKey: [slug, 'orgDefaults'],
    queryFn: () => getOrgDefaults(),
    enabled: !!slug,
  });
}

export function useUpdateOrgDefaults() {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: (input: OrgDefaults) => updateOrgDefaults(input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [slug, 'orgDefaults'] });
      toast.success(i18n.t('orgSettings.defaults.saveSuccess'));
    },
    onError: (err) => {
      toast.error(getErrorMessage(err, i18n.t('orgSettings.defaults.saveError')));
    },
  });
}
