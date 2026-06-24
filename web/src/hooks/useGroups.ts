import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { listGroups, createGroup, updateGroup, deleteGroup } from '../api/groups';
import type { CreateGroupRequest, UpdateGroupRequest } from '../types';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';
import i18n from '../i18n';

export function useGroups(page = 1, pageSize = 20) {
  return useQuery({
    queryKey: ['groups', page, pageSize],
    queryFn: () => listGroups(page, pageSize),
  });
}

export function useCreateGroup() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateGroupRequest) => createGroup(input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['groups'] });
      toast.success(i18n.t('toasts.groupCreated'));
    },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.groupCreateFailed'))); },
  });
}

export function useUpdateGroup() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateGroupRequest }) => updateGroup(id, input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['groups'] });
      toast.success(i18n.t('toasts.groupUpdated'));
    },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.groupUpdateFailed'))); },
  });
}

export function useDeleteGroup() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => deleteGroup(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['groups'] });
      toast.success(i18n.t('toasts.groupDeleted'));
    },
    onError: (err) => { toast.error(getErrorMessage(err, i18n.t('toasts.groupDeleteFailed'))); },
  });
}
