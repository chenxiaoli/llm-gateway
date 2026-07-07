import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { listChannelModelsByProvider, createChannelModel, updateChannelModel, deleteChannelModel } from '../api/channelModels';
import type { CreateChannelModelRequest, UpdateChannelModelRequest } from '../types';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';
import i18n from '../i18n';
import { useAuthStore } from '../stores/authStore';

export function useChannelModels(providerId: string) {
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useQuery({
    queryKey: [slug, 'providers', providerId, 'channelModels'],
    queryFn: () => listChannelModelsByProvider(providerId),
    enabled: !!slug && !!providerId,
  });
}

export function useCreateChannelModel(providerId: string) {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: (input: CreateChannelModelRequest) => createChannelModel(providerId, input),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: [slug, 'providers', providerId, 'channelModels'] }); toast.success(i18n.t('toasts.providerModelAdded')); },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.providerModelAddFailed'))); },
  });
}

export function useUpdateChannelModel(providerId: string) {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateChannelModelRequest }) =>
      updateChannelModel(id, input),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: [slug, 'providers', providerId, 'channelModels'] }); toast.success(i18n.t('toasts.providerModelUpdated')); },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.providerModelUpdateFailed'))); },
  });
}

export function useDeleteChannelModel(providerId: string) {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: (id: string) => deleteChannelModel(id),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: [slug, 'providers', providerId, 'channelModels'] }); toast.success(i18n.t('toasts.providerModelRemoved')); },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.providerModelRemoveFailed'))); },
  });
}
