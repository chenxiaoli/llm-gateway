import { adminApiClient, apiClient } from './client';
import type {
  Account,
  AccountBalanceResponse,
  CreateTransactionRequest,
  MeBalanceResponse,
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
