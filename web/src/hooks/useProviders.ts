import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { listProviders, getProvider, createProvider, updateProvider, deleteProvider } from '../api/providers';
import type { CreateProviderRequest, UpdateProviderRequest } from '../types';
import { message } from 'antd';

export function useProviders() {
  return useQuery({ queryKey: ['providers'], queryFn: listProviders });
}

export function useProvider(id: string) {
  return useQuery({ queryKey: ['providers', id], queryFn: () => getProvider(id), enabled: !!id });
}

export function useCreateProvider() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateProviderRequest) => createProvider(input),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['providers'] }); message.success('Provider created'); },
    onError: () => { message.error('Failed to create provider'); },
  });
}

export function useUpdateProvider() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateProviderRequest }) => updateProvider(id, input),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: ['providers'] });
      queryClient.invalidateQueries({ queryKey: ['providers', variables.id] });
      message.success('Provider updated');
    },
    onError: () => { message.error('Failed to update provider'); },
  });
}

export function useDeleteProvider() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => deleteProvider(id),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['providers'] }); message.success('Provider deleted'); },
    onError: () => { message.error('Failed to delete provider'); },
  });
}
