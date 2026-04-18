import { adminApiClient } from './client';
import type { PricingPolicyWithCounts, CreatePricingPolicy } from '../types';

export async function listPricingPolicies(): Promise<PricingPolicyWithCounts[]> {
  const { data } = await adminApiClient.get<PricingPolicyWithCounts[]>('/pricing-policies');
  return data;
}

export async function createPricingPolicy(input: CreatePricingPolicy): Promise<PricingPolicyWithCounts> {
  const { data } = await adminApiClient.post<PricingPolicyWithCounts>('/pricing-policies', input);
  return data;
}
