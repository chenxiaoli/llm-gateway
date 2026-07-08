import { apiClient } from './client';
import type {
  Invitation,
  InvitationPreview,
  CreateInvitationBody,
  AcceptInvitationBody,
  AuthResponse,
} from '../types';

export async function listInvitations(orgSlug: string): Promise<Invitation[]> {
  const r = await apiClient.get<Invitation[]>(`/${orgSlug}/invitations`);
  return r.data;
}

export async function createInvitation(
  orgSlug: string,
  body: CreateInvitationBody,
): Promise<Invitation> {
  const r = await apiClient.post<Invitation>(`/${orgSlug}/invitations`, body);
  return r.data;
}

export async function revokeInvitation(
  orgSlug: string,
  id: string,
): Promise<void> {
  await apiClient.delete(`/${orgSlug}/invitations/${id}`);
}

export async function previewInvitation(token: string): Promise<InvitationPreview> {
  const r = await apiClient.get<InvitationPreview>(
    '/invitations/preview',
    { params: { token } },
  );
  return r.data;
}

export async function acceptInvitation(body: AcceptInvitationBody): Promise<AuthResponse> {
  const r = await apiClient.post<AuthResponse>('/invitations/accept', body);
  return r.data;
}
