import { apiClient, orgPrefix } from './client';
import type {
  Account,
  AccountBalanceResponse,
  CreateTransactionRequest,
  MeBalanceResponse,
  RequestDetailsResponse,
  UpdateThresholdRequest,
} from '../types';

export async function getUserBalance(
  userId: string,
  page = 1,
  pageSize = 20
): Promise<AccountBalanceResponse> {
  const response = await apiClient.get<AccountBalanceResponse>(
    `${orgPrefix()}/admin/users/${userId}/balance`,
    { params: { page, page_size: pageSize } }
  );
  return response.data;
}

export async function rechargeUser(
  userId: string,
  data: CreateTransactionRequest
): Promise<Account> {
  const response = await apiClient.post<Account>(
    `${orgPrefix()}/admin/users/${userId}/recharge`,
    { ...data, type: 'credit' as const }
  );
  return response.data;
}

export async function adjustUserBalance(
  userId: string,
  data: CreateTransactionRequest
): Promise<Account> {
  const response = await apiClient.post<Account>(
    `${orgPrefix()}/admin/users/${userId}/adjust`,
    data
  );
  return response.data;
}

export async function updateUserThreshold(
  userId: string,
  data: UpdateThresholdRequest
): Promise<Account> {
  const response = await apiClient.patch<Account>(
    `${orgPrefix()}/admin/users/${userId}/threshold`,
    data
  );
  return response.data;
}

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
