import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { listKeys, getKey, createKey, updateKey, deleteKey } from '../api/keys';
import type { CreateKeyRequest, UpdateKeyRequest } from '../types';
import { message } from 'antd';

export function useKeys() {
  return useQuery({ queryKey: ['keys'], queryFn: listKeys });
}

export function useKey(id: string) {
  return useQuery({ queryKey: ['keys', id], queryFn: () => getKey(id), enabled: !!id });
}

export function useCreateKey() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateKeyRequest) => createKey(input),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['keys'] }); message.success('API key created'); },
    onError: () => { message.error('Failed to create API key'); },
  });
}

export function useUpdateKey() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateKeyRequest }) => updateKey(id, input),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['keys'] }); message.success('API key updated'); },
    onError: () => { message.error('Failed to update API key'); },
  });
}

export function useDeleteKey() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => deleteKey(id),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['keys'] }); message.success('API key deleted'); },
    onError: () => { message.error('Failed to delete API key'); },
  });
}
