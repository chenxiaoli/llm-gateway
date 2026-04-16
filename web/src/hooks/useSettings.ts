import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { getSettings, updateSettings } from '../api/settings';
import type { UpdateSettingsRequest } from '../types';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';

export function useSettings() {
  return useQuery({ queryKey: ['settings'], queryFn: getSettings });
}

export function useUpdateSettings() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: UpdateSettingsRequest) => updateSettings(input),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['settings'] }); toast.success('Settings updated'); },
    onError: (err) => { toast.error(getErrorMessage(err, 'Failed to update settings')); },
  });
}
