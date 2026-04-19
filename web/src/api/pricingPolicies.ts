import { adminApiClient } from './client';
import type { PricingPolicyWithCounts, CreatePricingPolicy, UpdatePricingPolicy } from '../types';

export async function listPricingPolicies(): Promise<PricingPolicyWithCounts[]> {
  const { data } = await adminApiClient.get<PricingPolicyWithCounts[]>('/pricing-policies');
  return data;
}

export async function createPricingPolicy(input: CreatePricingPolicy): Promise<PricingPolicyWithCounts> {
  const { data } = await adminApiClient.post<PricingPolicyWithCounts>('/pricing-policies', input);
  return data;
}

export async function updatePricingPolicy(id: string, input: UpdatePricingPolicy): Promise<PricingPolicyWithCounts> {
  const { data } = await adminApiClient.patch<PricingPolicyWithCounts>(`/pricing-policies/${id}`, input);
  return data;
}

export async function deletePricingPolicy(id: string): Promise<void> {
  await adminApiClient.delete(`/pricing-policies/${id}`);
}
