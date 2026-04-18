import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { listPricingPolicies, createPricingPolicy, updatePricingPolicy, deletePricingPolicy } from '../api/pricingPolicies';
import type { CreatePricingPolicy, UpdatePricingPolicy } from '../types';
import { toast } from 'sonner';
import { getErrorMessage } from '../api/client';

export function usePricingPolicies() {
  return useQuery({ queryKey: ['pricing-policies'], queryFn: listPricingPolicies });
}

export function useCreatePricingPolicy() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreatePricingPolicy) => createPricingPolicy(input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['pricing-policies'] });
      toast.success('Pricing policy created');
    },
    onError: (err) => {
      toast.error(getErrorMessage(err, 'Failed to create pricing policy'));
    },
  });
}

export function useUpdatePricingPolicy() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdatePricingPolicy }) => updatePricingPolicy(id, input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['pricing-policies'] });
      toast.success('Pricing policy updated');
    },
    onError: (err) => {
      toast.error(getErrorMessage(err, 'Failed to update pricing policy'));
    },
  });
}

export function useDeletePricingPolicy() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => deletePricingPolicy(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['pricing-policies'] });
      toast.success('Pricing policy deleted');
    },
    onError: (err) => {
      toast.error(getErrorMessage(err, 'Failed to delete pricing policy'));
    },
  });
}
