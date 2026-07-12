import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  adjustMemberBalance,
  changeMemberRole,
  inviteMember,
  listMembers,
  rechargeMember,
  removeMember,
  setMemberThreshold,
  updateMember,
} from '../api/members';
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

// useUpdateMember — partial {role?, enabled?, group_id?} update.
export function useUpdateMember() {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: ({ userId, data }: {
      userId: string;
      data: { role?: MemberRole; enabled?: boolean; group_id?: string | null };
    }) => updateMember(userId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [slug, 'members'] });
      toast.success(i18n.t('members.toasts.updated'));
    },
    onError: (err) => {
      toast.error(getErrorMessage(err, i18n.t('members.toasts.updateFailed')));
    },
  });
}

export function useRechargeMember() {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: ({ userId, data }: {
      userId: string;
      data: { amount: number; description?: string };
    }) => rechargeMember(userId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [slug, 'members'] });
      queryClient.invalidateQueries({ queryKey: [slug, 'member-balance'] });
      toast.success(i18n.t('members.toasts.recharged'));
    },
    onError: (err) => {
      toast.error(getErrorMessage(err, i18n.t('members.toasts.rechargeFailed')));
    },
  });
}

export function useAdjustMemberBalance() {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: ({ userId, data }: {
      userId: string;
      data: {
        type: 'credit_adjustment' | 'debit_refund';
        amount: number;
        description?: string;
      };
    }) => adjustMemberBalance(userId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [slug, 'members'] });
      queryClient.invalidateQueries({ queryKey: [slug, 'member-balance'] });
      toast.success(i18n.t('members.toasts.adjusted'));
    },
    onError: (err) => {
      toast.error(getErrorMessage(err, i18n.t('members.toasts.adjustFailed')));
    },
  });
}

export function useSetMemberThreshold() {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: ({ userId, threshold }: { userId: string; threshold: number }) =>
      setMemberThreshold(userId, threshold),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [slug, 'members'] });
      queryClient.invalidateQueries({ queryKey: [slug, 'member-balance'] });
      toast.success(i18n.t('members.toasts.thresholdUpdated'));
    },
    onError: (err) => {
      toast.error(getErrorMessage(err, i18n.t('members.toasts.thresholdUpdateFailed')));
    },
  });
}
