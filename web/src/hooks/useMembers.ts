import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { listMembers, inviteMember, changeMemberRole, removeMember } from '../api/members';
import type { MemberRole } from '../types';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';
import i18n from '../i18n';
import { useAuthStore } from '../stores/authStore';

export function useMembers() {
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useQuery({
    queryKey: [slug, 'members'],
    queryFn: () => listMembers(),
    enabled: !!slug,
  });
}

export function useInviteMember() {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: (req: { username: string; role: MemberRole }) => inviteMember(req),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [slug, 'members'] });
      toast.success(i18n.t('members.toasts.invited'));
    },
    onError: (err) => {
      toast.error(getErrorMessage(err, i18n.t('members.toasts.inviteFailed')));
    },
  });
}

export function useChangeMemberRole() {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: ({ userId, role }: { userId: string; role: MemberRole }) =>
      changeMemberRole(userId, role),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [slug, 'members'] });
      toast.success(i18n.t('members.toasts.roleUpdated'));
    },
    onError: (err) => {
      toast.error(getErrorMessage(err, i18n.t('members.toasts.roleUpdateFailed')));
    },
  });
}

export function useRemoveMember() {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: (userId: string) => removeMember(userId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [slug, 'members'] });
      toast.success(i18n.t('members.toasts.removed'));
    },
    onError: (err) => {
      toast.error(getErrorMessage(err, i18n.t('members.toasts.removeFailed')));
    },
  });
}
