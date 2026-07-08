import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { listAllChannels, listChannels, createChannel as createChannelApi, updateChannel as updateChannelApi, deleteChannel as deleteChannelApi, getChannel, listChannelModelsByChannel, createChannelModelByChannel, updateChannelModel, deleteChannelModel, updateChannelApiKey, listProviderModels, testChannel } from '../api/providers';
import type { CreateChannelRequest, UpdateChannelRequest, CreateChannelModelRequest, UpdateChannelModelRequest } from '../types';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';
import i18n from '../i18n';
import { useAuthStore } from '../stores/authStore';

export function useAllChannels() {
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useQuery({ queryKey: [slug, 'channels'], queryFn: listAllChannels, enabled: !!slug });
}

export function useToggleChannel() {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: ({ id, enabled }: { id: string; enabled: boolean }) => updateChannelApi(id, { enabled }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [slug, 'channels'] });
    },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.channelUpdateFailed'))); },
  });
}

export function useProviderModels(providerId: string) {
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useQuery({
    queryKey: [slug, 'provider-models', providerId],
    queryFn: () => listProviderModels(providerId),
    enabled: !!slug && !!providerId,
  });
}

export function useChannel(id: string) {
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useQuery({ queryKey: [slug, 'channels', id], queryFn: () => getChannel(id), enabled: !!slug && !!id });
}

export function useChannels(providerId: string) {
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useQuery({
    queryKey: [slug, 'providers', providerId, 'channels'],
    queryFn: () => listChannels(providerId),
    enabled: !!slug && !!providerId,
  });
}

export function useCreateChannel(providerId: string) {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: (input: CreateChannelRequest) => createChannelApi({ ...input, provider_id: providerId }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [slug, 'providers', providerId, 'channels'] });
      queryClient.invalidateQueries({ queryKey: [slug, 'channels'] });
      toast.success(i18n.t('toasts.channelCreated'));
    },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.channelCreateFailed'))); },
  });
}

export function useUpdateChannel(providerId: string) {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateChannelRequest }) => updateChannelApi(id, input),
    onSuccess: (_data, { id }) => {
      queryClient.invalidateQueries({ queryKey: [slug, 'channels', id] });
      queryClient.invalidateQueries({ queryKey: [slug, 'providers', providerId, 'channels'] });
      toast.success(i18n.t('toasts.channelUpdated'));
    },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.channelUpdateFailed'))); },
  });
}

export function useDeleteChannel(providerId: string) {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: (id: string) => deleteChannelApi(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [slug, 'providers', providerId, 'channels'] });
      toast.success(i18n.t('toasts.channelDeleted'));
    },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.channelDeleteFailed'))); },
  });
}

export function useChannelModels(channelId: string) {
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useQuery({
    queryKey: [slug, 'channel-models', channelId],
    queryFn: () => listChannelModelsByChannel(channelId),
    enabled: !!slug && !!channelId,
  });
}

export function useCreateChannelModel(channelId: string) {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: (input: CreateChannelModelRequest) => createChannelModelByChannel(channelId, input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [slug, 'channel-models', channelId] });
      toast.success(i18n.t('toasts.channelModelCreated'));
    },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.channelModelCreateFailed'))); },
  });
}

export function useUpdateChannelModel(channelId: string) {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateChannelModelRequest }) => updateChannelModel(id, input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [slug, 'channel-models', channelId] });
      toast.success(i18n.t('toasts.channelModelUpdated'));
    },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.channelModelUpdateFailed'))); },
  });
}

export function useDeleteChannelModel(channelId: string) {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: (id: string) => deleteChannelModel(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [slug, 'channel-models', channelId] });
      toast.success(i18n.t('toasts.channelModelDeleted'));
    },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.channelModelDeleteFailed'))); },
  });
}

export function useUpdateChannelApiKey(channelId: string) {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: (api_key: string) => updateChannelApiKey(channelId, { api_key }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [slug, 'channels', channelId] });
      queryClient.invalidateQueries({ queryKey: [slug, 'channels'] });
      toast.success(i18n.t('toasts.channelKeyUpdated'));
    },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.channelKeyUpdateFailed'))); },
  });
}

export function useTestChannel() {
  return useMutation({
    mutationFn: ({ id, endpointKey, stream }: { id: string; endpointKey?: string; stream?: boolean }) => testChannel(id, endpointKey, stream),
  });
}
