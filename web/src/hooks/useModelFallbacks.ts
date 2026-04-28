import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { listModelFallbacks, createModelFallback, updateModelFallback, deleteModelFallback } from '../api/modelFallbacks';
import type { CreateModelFallbackRequest, UpdateModelFallbackRequest } from '../types';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';

export function useModelFallbacks() {
  return useQuery({ queryKey: ['model-fallbacks'], queryFn: listModelFallbacks });
}

export function useCreateModelFallback() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateModelFallbackRequest) => createModelFallback(input),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['model-fallbacks'] }); toast.success('Model fallback created'); },
    onError: (err) => { toast.error(getErrorMessage(err, 'Failed to create model fallback')); },
  });
}

export function useUpdateModelFallback() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateModelFallbackRequest }) => updateModelFallback(id, input),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['model-fallbacks'] }); toast.success('Model fallback updated'); },
    onError: (err) => { toast.error(getErrorMessage(err, 'Failed to update model fallback')); },
  });
}

export function useDeleteModelFallback() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => deleteModelFallback(id),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['model-fallbacks'] }); toast.success('Model fallback deleted'); },
    onError: (err) => { toast.error(getErrorMessage(err, 'Failed to delete model fallback')); },
  });
}
