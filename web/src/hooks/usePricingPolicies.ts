import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { listPricingPolicies, createPricingPolicy } from '../api/pricingPolicies';
import type { CreatePricingPolicy } from '../types';
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
