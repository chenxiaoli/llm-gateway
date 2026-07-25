import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  getMemberBalance,
  rechargeMember,
  adjustMemberBalance,
  setMemberThreshold,
} from '../api/members';
import { getMyBalance, getRequestDetails } from '../api/accounts';
import type { CreateTransactionRequest, UpdateThresholdRequest } from '../types';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';
import i18n from '../i18n';
import { useAuthStore } from '../stores/authStore';

// --- Legacy per-user account hooks ---
// These wrap the new /admin/members/{user_id}/* routes (added in Task 6) via
// api/members. They retain their old names + signatures so pages/Users.tsx
// and pages/AccountBalance.tsx keep compiling; Task 11 deletes Users.tsx and
// a follow-up will migrate AccountBalance.tsx to the new useMembers hooks.

export function useUserBalance(userId: string, page = 1, pageSize = 20) {
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useQuery({
    queryKey: [slug, 'user-balance', userId, page, pageSize],
    queryFn: () => getMemberBalance(userId, page, pageSize),
    enabled: !!slug && !!userId,
  });
}

export function useRechargeUser() {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: ({
      userId,
      data,
    }: {
      userId: string;
      data: CreateTransactionRequest;
    }) =>
      rechargeMember(userId, {
        // Forward the caller's type so the "Deduct" path can route through
        // /recharge with type:'debit'. Defaults to 'credit' if the caller
        // didn't set one (kept inside rechargeMember for the no-arg case).
        type: data.type === 'debit' ? 'debit' : 'credit',
        amount: data.amount,
        description: data.description,
      }),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({
        queryKey: [slug, 'user-balance', variables.userId],
      });
      toast.success(i18n.t('toasts.balanceRecharged'));
    },
    onError: (err) => {
      toast.error(getErrorMessage(err, i18n.t('toasts.balanceRechargeFailed')));
    },
  });
}

export function useAdjustUser() {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: ({
      userId,
      data,
    }: {
      userId: string;
      data: CreateTransactionRequest;
    }) =>
      adjustMemberBalance(userId, {
        type: data.type as 'credit_adjustment' | 'debit_refund',
        amount: data.amount,
        description: data.description,
      }),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({
        queryKey: [slug, 'user-balance', variables.userId],
      });
      toast.success(i18n.t('toasts.balanceAdjusted'));
    },
    onError: (err) => {
      toast.error(getErrorMessage(err, i18n.t('toasts.balanceAdjustFailed')));
    },
  });
}

export function useUpdateThreshold() {
  const queryClient = useQueryClient();
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useMutation({
    mutationFn: ({
      userId,
      data,
    }: {
      userId: string;
      data: UpdateThresholdRequest;
    }) => setMemberThreshold(userId, data.threshold),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({
        queryKey: [slug, 'user-balance', variables.userId],
      });
      toast.success(i18n.t('toasts.thresholdUpdated'));
    },
    onError: (err) => {
      toast.error(getErrorMessage(err, i18n.t('toasts.thresholdUpdateFailed')));
    },
  });
}

export function useMyBalance(page = 1, pageSize = 20) {
  return useQuery({
    queryKey: ['my-balance', page, pageSize],
    queryFn: () => getMyBalance(page, pageSize),
  });
}

export function useRequestDetails(requestId: string | null) {
  const slug = useAuthStore((s) => s.currentOrg?.slug) ?? '';
  return useQuery({
    queryKey: [slug, 'request-details', requestId],
    queryFn: () => getRequestDetails(requestId!),
    enabled: !!slug && !!requestId,
  });
}
