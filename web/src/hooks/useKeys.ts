import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { listKeys, getKey, createKey, updateKey, deleteKey } from '../api/keys';
import type { CreateKeyRequest, UpdateKeyRequest } from '../types';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';
import i18n from '../i18n';
import { useAuthStore } from '../stores/authStore';

export function useKeys(page = 1, pageSize = 20) {
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useQuery({
    queryKey: [slug, 'keys', page, pageSize],
    queryFn: () => listKeys(page, pageSize),
    enabled: !!slug,
  });
}

export function useKey(id: string) {
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useQuery({
    queryKey: [slug, 'keys', id],
    queryFn: () => getKey(id),
    enabled: !!slug && !!id,
  });
}

export function useCreateKey() {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: (input: CreateKeyRequest) => createKey(input),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: [slug, 'keys'] }); toast.success(i18n.t('toasts.keyCreated')); },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.keyCreateFailed'))); },
  });
}

export function useUpdateKey() {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateKeyRequest }) => updateKey(id, input),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: [slug, 'keys'] }); toast.success(i18n.t('toasts.keyUpdated')); },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.keyUpdateFailed'))); },
  });
}

export function useDeleteKey() {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: (id: string) => deleteKey(id),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: [slug, 'keys'] }); toast.success(i18n.t('toasts.keyDeleted')); },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.keyDeleteFailed'))); },
  });
}
