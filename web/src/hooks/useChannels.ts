import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { listAllChannels, listChannels, createChannel as createChannelApi, updateChannel as updateChannelApi, deleteChannel as deleteChannelApi, getChannel, listChannelModelsByChannel, createChannelModelByChannel, updateChannelModel, deleteChannelModel } from '../api/providers';
import type { CreateChannelRequest, UpdateChannelRequest, CreateChannelModelRequest, UpdateChannelModelRequest } from '../types';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';

export function useAllChannels() {
  return useQuery({ queryKey: ['channels'], queryFn: listAllChannels });
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
      toast.success('Channel model created');
    },
    onError: (err) => { toast.error(getErrorMessage(err, 'Failed to create channel model')); },
  });
}

export function useUpdateChannelModel(channelId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateChannelModelRequest }) => updateChannelModel(id, input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['channel-models', channelId] });
      toast.success('Channel model updated');
    },
    onError: (err) => { toast.error(getErrorMessage(err, 'Failed to update channel model')); },
  });
}

export function useDeleteChannelModel(channelId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => deleteChannelModel(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['channel-models', channelId] });
      toast.success('Channel model deleted');
    },
    onError: (err) => { toast.error(getErrorMessage(err, 'Failed to delete channel model')); },
  });
}