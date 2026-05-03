import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { listKeys, getKey, createKey, updateKey, deleteKey } from '../api/keys';
import type { CreateKeyRequest, UpdateKeyRequest } from '../types';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';
import i18n from '../i18n';

export function useKeys(page = 1, pageSize = 20) {
  return useQuery({ queryKey: ['keys', page, pageSize], queryFn: () => listKeys(page, pageSize) });
}

export function useKey(id: string) {
  return useQuery({ queryKey: ['keys', id], queryFn: () => getKey(id), enabled: !!id });
}

export function useCreateKey() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateKeyRequest) => createKey(input),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['keys'] }); toast.success(i18n.t('toasts.keyCreated')); },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.keyCreateFailed'))); },
  });
}

export function useUpdateKey() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateKeyRequest }) => updateKey(id, input),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['keys'] }); toast.success(i18n.t('toasts.keyUpdated')); },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.keyUpdateFailed'))); },
  });
}

export function useDeleteKey() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => deleteKey(id),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['keys'] }); toast.success(i18n.t('toasts.keyDeleted')); },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.keyDeleteFailed'))); },
  });
}
