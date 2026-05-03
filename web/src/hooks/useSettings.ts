import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { getSettings, updateSettings, getSystemInfo } from '../api/settings';
import type { UpdateSettingsRequest } from '../types';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';
import i18n from '../i18n';

export function useSettings() {
  return useQuery({ queryKey: ['settings'], queryFn: getSettings });
}

export function useUpdateSettings() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: UpdateSettingsRequest) => updateSettings(input),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['settings'] }); toast.success(i18n.t('toasts.settingsUpdated')); },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.settingsUpdateFailed'))); },
  });
}

export function useSystemInfo() {
  return useQuery({ queryKey: ['system-info'], queryFn: getSystemInfo });
}
