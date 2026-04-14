import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { listProviders, getProvider, createProvider, updateProvider, deleteProvider, listChannels, createChannel as createChannelApi, updateChannel as updateChannelApi, deleteChannel as deleteChannelApi, syncModels } from '../api/providers';
import type { CreateProviderRequest, UpdateProviderRequest, CreateChannelRequest, UpdateChannelRequest } from '../types';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';

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
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['providers'] }); toast.success('Provider created'); },
    onError: (err) => { toast.error(getErrorMessage(err, 'Failed to create provider')); },
  });
}

export function useUpdateProvider() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateProviderRequest }) => updateProvider(id, input),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: ['providers'] });
      queryClient.invalidateQueries({ queryKey: ['providers', variables.id] });
      toast.success('Provider updated');
    },
    onError: (err) => { toast.error(getErrorMessage(err, 'Failed to update provider')); },
  });
}

export function useDeleteProvider() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => deleteProvider(id),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['providers'] }); toast.success('Provider deleted'); },
    onError: (err) => { toast.error(getErrorMessage(err, 'Failed to delete provider')); },
  });
}

export function useChannels(providerId: string) {
  return useQuery({
    queryKey: ['providers', providerId, 'channels'],
    queryFn: () => listChannels(providerId),
    enabled: !!providerId,
  });
}

export function useCreateChannel(providerId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateChannelRequest) => createChannelApi(providerId, input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['providers', providerId, 'channels'] });
      toast.success('Channel created');
    },
    onError: (err) => { toast.error(getErrorMessage(err, 'Failed to create channel')); },
  });
}

export function useUpdateChannel(providerId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateChannelRequest }) => updateChannelApi(id, input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['providers', providerId, 'channels'] });
      toast.success('Channel updated');
    },
    onError: (err) => { toast.error(getErrorMessage(err, 'Failed to update channel')); },
  });
}

export function useDeleteChannel(providerId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => deleteChannelApi(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['providers', providerId, 'channels'] });
      toast.success('Channel deleted');
    },
    onError: (err) => { toast.error(getErrorMessage(err, 'Failed to delete channel')); },
  });
}

export function useSyncModels(providerId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => syncModels(providerId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['providers', providerId, 'models'] });
      toast.success('Models synced');
    },
    onError: (err) => { toast.error(getErrorMessage(err, 'Failed to sync models')); },
  });
}
