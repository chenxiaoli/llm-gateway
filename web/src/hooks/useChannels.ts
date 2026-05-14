import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { listAllChannels, listChannels, createChannel as createChannelApi, updateChannel as updateChannelApi, deleteChannel as deleteChannelApi, getChannel, listChannelModelsByChannel, createChannelModelByChannel, updateChannelModel, deleteChannelModel, updateChannelApiKey, listProviderModels, testChannel } from '../api/providers';
import type { CreateChannelRequest, UpdateChannelRequest, CreateChannelModelRequest, UpdateChannelModelRequest } from '../types';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';
import i18n from '../i18n';

export function useAllChannels() {
  return useQuery({ queryKey: ['channels'], queryFn: listAllChannels });
}

export function useToggleChannel() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, enabled }: { id: string; enabled: boolean }) => updateChannelApi(id, { enabled }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['channels'] });
    },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.channelUpdateFailed'))); },
  });
}

export function useProviderModels(providerId: string) {
  return useQuery({
    queryKey: ['provider-models', providerId],
    queryFn: () => listProviderModels(providerId),
    enabled: !!providerId,
  });
}

export function useChannel(id: string) {
  return useQuery({ queryKey: ['channels', id], queryFn: () => getChannel(id), enabled: !!id });
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
    mutationFn: (input: CreateChannelRequest) => createChannelApi({ ...input, provider_id: providerId }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['providers', providerId, 'channels'] });
      queryClient.invalidateQueries({ queryKey: ['channels'] });
      toast.success(i18n.t('toasts.channelCreated'));
    },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.channelCreateFailed'))); },
  });
}

export function useUpdateChannel(providerId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateChannelRequest }) => updateChannelApi(id, input),
    onSuccess: (_data, { id }) => {
      queryClient.invalidateQueries({ queryKey: ['channels', id] });
      queryClient.invalidateQueries({ queryKey: ['providers', providerId, 'channels'] });
      toast.success(i18n.t('toasts.channelUpdated'));
    },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.channelUpdateFailed'))); },
  });
}

export function useDeleteChannel(providerId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => deleteChannelApi(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['providers', providerId, 'channels'] });
      toast.success(i18n.t('toasts.channelDeleted'));
    },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.channelDeleteFailed'))); },
  });
}

export function useChannelModels(channelId: string) {
  return useQuery({
    queryKey: ['channel-models', channelId],
    queryFn: () => listChannelModelsByChannel(channelId),
    enabled: !!channelId,
  });
}

export function useCreateChannelModel(channelId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateChannelModelRequest) => createChannelModelByChannel(channelId, input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['channel-models', channelId] });
      toast.success(i18n.t('toasts.channelModelCreated'));
    },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.channelModelCreateFailed'))); },
  });
}

export function useUpdateChannelModel(channelId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateChannelModelRequest }) => updateChannelModel(id, input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['channel-models', channelId] });
      toast.success(i18n.t('toasts.channelModelUpdated'));
    },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.channelModelUpdateFailed'))); },
  });
}

export function useDeleteChannelModel(channelId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => deleteChannelModel(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['channel-models', channelId] });
      toast.success(i18n.t('toasts.channelModelDeleted'));
    },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.channelModelDeleteFailed'))); },
  });
}

export function useUpdateChannelApiKey(channelId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (api_key: string) => updateChannelApiKey(channelId, { api_key }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['channels', channelId] });
      queryClient.invalidateQueries({ queryKey: ['channels'] });
      toast.success(i18n.t('toasts.channelKeyUpdated'));
    },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.channelKeyUpdateFailed'))); },
  });
}

export function useTestChannel() {
  return useMutation({
    mutationFn: ({ id, endpointKey }: { id: string; endpointKey?: string }) => testChannel(id, endpointKey),
  });
}