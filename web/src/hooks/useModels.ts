import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { listModels, listAllModels, createModel, createGlobalModel, updateModel, deleteModel } from '../api/models';
import type { CreateModelRequest, CreateGlobalModelRequest, UpdateModelRequest } from '../types';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';

export function useModels(providerId: string) {
  return useQuery({
    queryKey: ['providers', providerId, 'models'],
    queryFn: () => listModels(providerId),
    enabled: !!providerId,
  });
}

export function useAllModels() {
  return useQuery({ queryKey: ['models'], queryFn: listAllModels });
}

export function useCreateGlobalModel() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateGlobalModelRequest) => createGlobalModel(input),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['models'] }); toast.success('Model added'); },
    onError: (err) => { toast.error(getErrorMessage(err, 'Failed to add model')); },
  });
}

export function useCreateModel(providerId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateModelRequest) => createModel(providerId, input),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['providers', providerId] }); toast.success('Model added'); },
    onError: (err) => { toast.error(getErrorMessage(err, 'Failed to add model')); },
  });
}

export function useUpdateModel(_providerId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ modelName, input }: { modelName: string; input: UpdateModelRequest }) =>
      updateModel(modelName, input),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['models'] }); toast.success('Model updated'); },
    onError: (err) => { toast.error(getErrorMessage(err, 'Failed to update model')); },
  });
}

export function useUpdateGlobalModel() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ modelName, input }: { modelName: string; input: UpdateModelRequest }) =>
      updateModel(modelName, input),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['models'] }); toast.success('Model updated'); },
    onError: (err) => { toast.error(getErrorMessage(err, 'Failed to update model')); },
  });
}

export function useDeleteModel(_providerId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (modelName: string) => deleteModel(modelName),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['models'] }); toast.success('Model deleted'); },
    onError: (err) => { toast.error(getErrorMessage(err, 'Failed to delete model')); },
  });
}
