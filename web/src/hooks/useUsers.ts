import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { listUsers, updateUser, deleteUser } from '../api/users';
import type { UpdateUserRequest } from '../types';
import { toast } from 'sonner';

export function useUsers(page = 1, pageSize = 20) {
  return useQuery({ queryKey: ['users', page, pageSize], queryFn: () => listUsers(page, pageSize) });
}

export function useUpdateUser() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateUserRequest }) => updateUser(id, input),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['users'] }); toast.success('User updated'); },
    onError: () => { toast.error('Failed to update user'); },
  });
}

export function useDeleteUser() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => deleteUser(id),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['users'] }); toast.success('User deleted'); },
    onError: () => { toast.error('Failed to delete user'); },
  });
}
