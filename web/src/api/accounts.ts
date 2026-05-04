import { adminApiClient, apiClient } from './client';
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
  const response = await adminApiClient.get<AccountBalanceResponse>(
    `/users/${userId}/balance`,
    { params: { page, page_size: pageSize } }
  );
  return response.data;
}

export async function rechargeUser(
  userId: string,
  data: CreateTransactionRequest
): Promise<Account> {
  const response = await adminApiClient.post<Account>(
    `/users/${userId}/recharge`,
    { ...data, type: 'credit' as const }
  );
  return response.data;
}

export async function adjustUserBalance(
  userId: string,
  data: CreateTransactionRequest
): Promise<Account> {
  const response = await adminApiClient.post<Account>(
    `/users/${userId}/adjust`,
    data
  );
  return response.data;
}

export async function updateUserThreshold(
  userId: string,
  data: UpdateThresholdRequest
): Promise<Account> {
  const response = await adminApiClient.patch<Account>(
    `/users/${userId}/threshold`,
    data
  );
  return response.data;
}

export async function getMyBalance(
  page = 1,
  pageSize = 20
): Promise<MeBalanceResponse> {
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
  const response = await adminApiClient.get<RequestDetailsResponse>(
    `/requests/${requestId}`
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
