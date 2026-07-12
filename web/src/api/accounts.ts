import { apiClient, orgPrefix } from './client';
import type {
  MeBalanceResponse,
  RequestDetailsResponse,
} from '../types';

export async function getMyBalance(
  page = 1,
  pageSize = 20
): Promise<MeBalanceResponse> {
  // Global route — not org-scoped (mounted outside management_router)
  const response = await apiClient.get<MeBalanceResponse>(
    '/auth/me/balance',
    { params: { page, page_size: pageSize } }
  );
  return response.data;
}

const UNITS_PER_USD = 100_000_000;
function unitsToUsd(units: number): number {
  return units / UNITS_PER_USD;
}

export async function getRequestDetails(
  requestId: string
): Promise<RequestDetailsResponse> {
  const response = await apiClient.get<RequestDetailsResponse>(
    `${orgPrefix()}/admin/requests/${requestId}`
  );
  const data = response.data;
  // Convert raw i64 monetary units to USD floats
  if (data.transaction) {
    data.transaction.amount = unitsToUsd(data.transaction.amount);
    data.transaction.balance_after = unitsToUsd(data.transaction.balance_after);
  }
  if (data.usage) {
    data.usage.cost = unitsToUsd(data.usage.cost);
  }
  return data;
}
