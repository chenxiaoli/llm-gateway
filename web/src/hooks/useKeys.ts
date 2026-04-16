import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { listKeys, getKey, createKey, updateKey, deleteKey } from '../api/keys';
import type { CreateKeyRequest, UpdateKeyRequest } from '../types';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';

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
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['keys'] }); toast.success('API key created'); },
    onError: (err) => { toast.error(getErrorMessage(err, 'Failed to create API key')); },
  });
}

export function useUpdateKey() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateKeyRequest }) => updateKey(id, input),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['keys'] }); toast.success('API key updated'); },
    onError: (err) => { toast.error(getErrorMessage(err, 'Failed to update API key')); },
  });
}

export function useDeleteKey() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => deleteKey(id),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['keys'] }); toast.success('API key deleted'); },
    onError: (err) => { toast.error(getErrorMessage(err, 'Failed to delete API key')); },
  });
}
