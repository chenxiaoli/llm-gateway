import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { listPricingPolicies, createPricingPolicy, updatePricingPolicy, deletePricingPolicy } from '../api/pricingPolicies';
import type { CreatePricingPolicy, UpdatePricingPolicy } from '../types';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';
import i18n from '../i18n';

export function usePricingPolicies() {
  return useQuery({ queryKey: ['pricing-policies'], queryFn: listPricingPolicies });
}

export function useCreatePricingPolicy() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreatePricingPolicy) => createPricingPolicy(input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['pricing-policies'] });
      toast.success(i18n.t('toasts.pricingCreated'));
    },
    onError: (err) => {
      toast.error(getErrorMessage(err, i18n.t('toasts.pricingCreateFailed')));
    },
  });
}

export function useUpdatePricingPolicy() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdatePricingPolicy }) => updatePricingPolicy(id, input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['pricing-policies'] });
      toast.success(i18n.t('toasts.pricingUpdated'));
    },
    onError: (err) => {
      toast.error(getErrorMessage(err, i18n.t('toasts.pricingUpdateFailed')));
    },
  });
}

export function useDeletePricingPolicy() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => deletePricingPolicy(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['pricing-policies'] });
      toast.success(i18n.t('toasts.pricingDeleted'));
    },
    onError: (err) => {
      toast.error(getErrorMessage(err, i18n.t('toasts.pricingDeleteFailed')));
    },
  });
}
