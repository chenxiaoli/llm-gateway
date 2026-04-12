import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { getSettings, updateSettings } from '../api/settings';
import type { UpdateSettingsRequest } from '../types';
import { message } from 'antd';

export function useSettings() {
  return useQuery({ queryKey: ['settings'], queryFn: getSettings });
}

export function useUpdateSettings() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: UpdateSettingsRequest) => updateSettings(input),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['settings'] }); message.success('Settings updated'); },
    onError: () => { message.error('Failed to update settings'); },
  });
}
