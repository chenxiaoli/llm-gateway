import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { listModelFallbacks, createModelFallback, updateModelFallback, deleteModelFallback } from '../api/modelFallbacks';
import type { CreateModelFallbackRequest, UpdateModelFallbackRequest } from '../types';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';
import i18n from '../i18n';

export function useModelFallbacks() {
  return useQuery({ queryKey: ['model-fallbacks'], queryFn: listModelFallbacks });
}

export function useCreateModelFallback() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateModelFallbackRequest) => createModelFallback(input),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['model-fallbacks'] }); toast.success(i18n.t('toasts.fallbackCreated')); },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.fallbackCreateFailed'))); },
  });
}

export function useUpdateModelFallback() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateModelFallbackRequest }) => updateModelFallback(id, input),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['model-fallbacks'] }); toast.success(i18n.t('toasts.fallbackUpdated')); },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.fallbackUpdateFailed'))); },
  });
}

export function useDeleteModelFallback() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => deleteModelFallback(id),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['model-fallbacks'] }); toast.success(i18n.t('toasts.fallbackDeleted')); },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.fallbackDeleteFailed'))); },
  });
}
