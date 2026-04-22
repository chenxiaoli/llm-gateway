import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  getUserBalance,
  rechargeUser,
  adjustUserBalance,
  updateUserThreshold,
  getMyBalance,
} from '../api/accounts';
import type { CreateTransactionRequest, UpdateThresholdRequest } from '../types';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';

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
      toast.success('Balance recharged successfully');
    },
    onError: (err) => {
      toast.error(getErrorMessage(err, 'Failed to recharge'));
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
      toast.success('Balance adjusted successfully');
    },
    onError: (err) => {
      toast.error(getErrorMessage(err, 'Failed to adjust balance'));
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
      toast.success('Threshold updated');
    },
    onError: (err) => {
      toast.error(getErrorMessage(err, 'Failed to update threshold'));
    },
  });
}

export function useMyBalance(page = 1, pageSize = 20) {
  return useQuery({
    queryKey: ['my-balance', page, pageSize],
    queryFn: () => getMyBalance(page, pageSize),
  });
}
