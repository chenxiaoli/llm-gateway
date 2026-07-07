import { apiClient, orgPrefix } from './client';
import type { PricingPolicyWithCounts, CreatePricingPolicy, UpdatePricingPolicy } from '../types';

export async function listPricingPolicies(): Promise<PricingPolicyWithCounts[]> {
  const { data } = await apiClient.get<PricingPolicyWithCounts[]>(`${orgPrefix()}/admin/pricing-policies`);
  return data;
}

export async function createPricingPolicy(input: CreatePricingPolicy): Promise<PricingPolicyWithCounts> {
  const { data } = await apiClient.post<PricingPolicyWithCounts>(`${orgPrefix()}/admin/pricing-policies`, input);
  return data;
}

export async function updatePricingPolicy(id: string, input: UpdatePricingPolicy): Promise<PricingPolicyWithCounts> {
  const { data } = await apiClient.patch<PricingPolicyWithCounts>(`${orgPrefix()}/admin/pricing-policies/${id}`, input);
  return data;
}

export async function deletePricingPolicy(id: string): Promise<void> {
  await apiClient.delete(`${orgPrefix()}/admin/pricing-policies/${id}`);
}
