import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { listChannelModelsByProvider, createChannelModel, updateChannelModel, deleteChannelModel } from '../api/channelModels';
import type { CreateChannelModelRequest, UpdateChannelModelRequest } from '../types';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';

export function useChannelModels(providerId: string) {
  return useQuery({
    queryKey: ['providers', providerId, 'channelModels'],
    queryFn: () => listChannelModelsByProvider(providerId),
    enabled: !!providerId,
  });
}

export function useCreateChannelModel(providerId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateChannelModelRequest) => createChannelModel(providerId, input),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['providers', providerId, 'channelModels'] }); toast.success('Model added to provider'); },
    onError: (err) => { toast.error(getErrorMessage(err, 'Failed to add model')); },
  });
}

export function useUpdateChannelModel(providerId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateChannelModelRequest }) =>
      updateChannelModel(id, input),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['providers', providerId, 'channelModels'] }); toast.success('Model updated'); },
    onError: (err) => { toast.error(getErrorMessage(err, 'Failed to update model')); },
  });
}

export function useDeleteChannelModel(providerId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => deleteChannelModel(id),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['providers', providerId, 'channelModels'] }); toast.success('Model removed'); },
    onError: (err) => { toast.error(getErrorMessage(err, 'Failed to remove model')); },
  });
}
