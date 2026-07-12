import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { listAllModels, createGlobalModel, updateModel, deleteModel } from '../api/models';
import type { CreateGlobalModelRequest, UpdateModelRequest } from '../types';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';
import i18n from '../i18n';
import { useAuthStore } from '../stores/authStore';

export function useAllModels() {
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useQuery({ queryKey: [slug, 'models'], queryFn: listAllModels, enabled: !!slug });
}

export function useCreateGlobalModel() {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: (input: CreateGlobalModelRequest) => createGlobalModel(input),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: [slug, 'models'] }); toast.success(i18n.t('toasts.modelAdded')); },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.modelAddFailed'))); },
  });
}

export function useUpdateGlobalModel() {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: ({ modelName, input }: { modelName: string; input: UpdateModelRequest }) =>
      updateModel(modelName, input),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: [slug, 'models'] }); toast.success(i18n.t('toasts.modelUpdated')); },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.modelUpdateFailed'))); },
  });
}

export function useDeleteModel() {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: (modelName: string) => deleteModel(modelName),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: [slug, 'models'] }); toast.success(i18n.t('toasts.modelDeleted')); },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.modelDeleteFailed'))); },
  });
}
