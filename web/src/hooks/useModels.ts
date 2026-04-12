import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { listModels, createModel, updateModel, deleteModel } from '../api/models';
import type { CreateModelRequest, UpdateModelRequest } from '../types';
import { message } from 'antd';

export function useModels(providerId: string) {
  return useQuery({
    queryKey: ['providers', providerId, 'models'],
    queryFn: () => listModels(providerId),
    enabled: !!providerId,
  });
}

export function useCreateModel(providerId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateModelRequest) => createModel(providerId, input),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['providers', providerId] }); message.success('Model added'); },
    onError: () => { message.error('Failed to add model'); },
  });
}

export function useUpdateModel(providerId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ modelName, input }: { modelName: string; input: UpdateModelRequest }) =>
      updateModel(providerId, modelName, input),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['providers', providerId] }); message.success('Model updated'); },
    onError: () => { message.error('Failed to update model'); },
  });
}

export function useDeleteModel(providerId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (modelName: string) => deleteModel(providerId, modelName),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['providers', providerId] }); message.success('Model deleted'); },
    onError: () => { message.error('Failed to delete model'); },
  });
}
