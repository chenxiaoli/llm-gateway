import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { getSettings, updateSettings, getSystemInfo, getNatsStatus } from '../api/settings';
import type { UpdateSettingsRequest } from '../types';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';
import i18n from '../i18n';
import { useAuthStore } from '../stores/authStore';

export function useSettings() {
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useQuery({ queryKey: [slug, 'settings'], queryFn: getSettings, enabled: !!slug });
}

export function useUpdateSettings() {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: (input: UpdateSettingsRequest) => updateSettings(input),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: [slug, 'settings'] }); toast.success(i18n.t('toasts.settingsUpdated')); },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.settingsUpdateFailed'))); },
  });
}

export function useSystemInfo() {
  return useQuery({ queryKey: ['system-info'], queryFn: getSystemInfo });
}

export function useNatsStatus() {
  return useQuery({ queryKey: ['nats-status'], queryFn: getNatsStatus });
}
