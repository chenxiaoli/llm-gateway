import { apiClient } from './client';
import type {
  Invitation,
  InvitationPreview,
  CreateInvitationBody,
  AcceptInvitationBody,
  AuthResponse,
} from '../types';

export async function listInvitations(orgSlug: string): Promise<Invitation[]> {
  const r = await apiClient.get<Invitation[]>(`/api/v1/${orgSlug}/invitations`);
  return r.data;
}

export async function createInvitation(
  orgSlug: string,
  body: CreateInvitationBody,
): Promise<Invitation> {
  const r = await apiClient.post<Invitation>(`/api/v1/${orgSlug}/invitations`, body);
  return r.data;
}

export async function revokeInvitation(
  orgSlug: string,
  id: string,
): Promise<void> {
  await apiClient.delete(`/api/v1/${orgSlug}/invitations/${id}`);
}

export async function previewInvitation(token: string): Promise<InvitationPreview> {
  const r = await apiClient.get<InvitationPreview>(
    '/api/v1/invitations/preview',
    { params: { token } },
  );
  return r.data;
}

export async function acceptInvitation(body: AcceptInvitationBody): Promise<AuthResponse> {
  const r = await apiClient.post<AuthResponse>('/api/v1/invitations/accept', body);
  return r.data;
}
