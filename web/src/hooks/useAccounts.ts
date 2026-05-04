import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  getUserBalance,
  rechargeUser,
  adjustUserBalance,
  updateUserThreshold,
  getMyBalance,
  getRequestDetails,
} from '../api/accounts';
import type { CreateTransactionRequest, UpdateThresholdRequest } from '../types';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';
import i18n from '../i18n';

export function useUserBalance(userId: string, page = 1, pageSize = 20) {
  return useQuery({
    queryKey: ['user-balance', userId, page, pageSize],
    queryFn: () => getUserBalance(userId, page, pageSize),
    enabled: !!userId,
  });
}

export function useRechargeUser() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      userId,
      data,
    }: {
      userId: string;
      data: CreateTransactionRequest;
    }) => rechargeUser(userId, data),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({
        queryKey: ['user-balance', variables.userId],
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
  return useMutation({
    mutationFn: ({
      userId,
      data,
    }: {
      userId: string;
      data: CreateTransactionRequest;
    }) => adjustUserBalance(userId, data),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({
        queryKey: ['user-balance', variables.userId],
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
  return useMutation({
    mutationFn: ({
      userId,
      data,
    }: {
      userId: string;
      data: UpdateThresholdRequest;
    }) => updateUserThreshold(userId, data),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({
        queryKey: ['user-balance', variables.userId],
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
  return useQuery({
    queryKey: ['request-details', requestId],
    queryFn: () => getRequestDetails(requestId!),
    enabled: !!requestId,
  });
}
