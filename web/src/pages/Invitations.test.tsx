import { describe, it, expect, beforeEach, vi } from 'vitest';
import { renderWithProviders } from '../test/render';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import Invitations from './Invitations';
import { useAuthStore } from '../stores/authStore';
import type { Invitation, OrgSummary } from '../types';

function mockClipboard() {
  Object.assign(navigator, {
    clipboard: {
      writeText: vi.fn().mockResolvedValue(undefined),
    },
  });
}

function isoDaysFromNow(days: number): string {
  return new Date(Date.now() + days * 86400000).toISOString();
}

const acmeOrg: OrgSummary = {
  id: 'org-acme',
  slug: 'acme',
  name: 'Acme',
  role: 'admin',
  group_id: null,
};

function seedAcme() {
  useAuthStore.setState({ currentOrg: acmeOrg });
}

const pendingInvitation: Invitation = {
  id: 'inv-pending',
  token: 'tok-pending',
  url: 'https://app.example.com/accept-invite?token=tok-pending',
  role: 'member',
  recipient_email: 'pending@example.com',
  created_at: '2026-07-01T00:00:00Z',
  expires_at: isoDaysFromNow(5),
  accepted_at: null,
  accepted_by: null,
  revoked_at: null,
};

const acceptedInvitation: Invitation = {
  id: 'inv-accepted',
  token: 'tok-accepted',
  url: 'https://app.example.com/accept-invite?token=tok-accepted',
  role: 'admin',
  recipient_email: 'accepted@example.com',
  created_at: '2026-07-01T00:00:00Z',
  expires_at: isoDaysFromNow(3),
  accepted_at: '2026-07-05T00:00:00Z',
  accepted_by: 'bob',
  revoked_at: null,
};

const revokedInvitation: Invitation = {
  id: 'inv-revoked',
  token: 'tok-revoked',
  url: 'https://app.example.com/accept-invite?token=tok-revoked',
  role: 'member',
  recipient_email: 'revoked@example.com',
  created_at: '2026-07-01T00:00:00Z',
  expires_at: isoDaysFromNow(7),
  accepted_at: null,
  accepted_by: null,
  revoked_at: '2026-07-03T00:00:00Z',
};

const ROUTE = '/acme/admin/invitations';

beforeEach(() => {
  mockClipboard();
  seedAcme();
});

describe('Invitations admin page', () => {
  it('empty state: shows the empty hint when there are no invitations', async () => {
    server.use(
      http.get('*/api/v1/acme/invitations', () => HttpResponse.json([])),
    );

    renderWithProviders(<Invitations />, { route: ROUTE });

    await waitFor(() => {
      expect(screen.getByText('No invitations yet')).toBeInTheDocument();
    });
  });

  it('generate flow: clicking Generate POSTs with role + recipient_email and refreshes the list', async () => {
    let createdBody: { role?: string; recipient_email?: string } | undefined;
    server.use(
      http.get('*/api/v1/acme/invitations', () => HttpResponse.json([])),
      http.post('*/api/v1/acme/invitations', async ({ request }) => {
        const body = (await request.json()) as { role: string; recipient_email: string };
        createdBody = body;
        return HttpResponse.json({
          ...pendingInvitation,
          id: 'inv-new',
          role: body.role,
          recipient_email: body.recipient_email,
        });
      }),
    );

    renderWithProviders(<Invitations />, { route: ROUTE });

    // Wait for list load
    await waitFor(() => {
      expect(screen.getByText('No invitations yet')).toBeInTheDocument();
    });

    // Switch role to admin via the Select
    await userEvent.selectOptions(screen.getByRole('combobox'), 'admin');

    // Fill the recipient email (Phase 4: required field)
    await userEvent.type(screen.getByLabelText(/Recipient email/i), 'alice@example.com');

    // Click Generate
    await userEvent.click(screen.getByRole('button', { name: 'Generate' }));

    await waitFor(() => {
      expect(createdBody?.role).toBe('admin');
      expect(createdBody?.recipient_email).toBe('alice@example.com');
    });
  });

  it('email required: Generate button stays disabled until recipient email is filled', async () => {
    server.use(
      http.get('*/api/v1/acme/invitations', () => HttpResponse.json([])),
    );

    renderWithProviders(<Invitations />, { route: ROUTE });

    await waitFor(() => {
      expect(screen.getByText('No invitations yet')).toBeInTheDocument();
    });

    // The email field is required — left empty, the button must be disabled.
    const emailInput = screen.getByLabelText(/Recipient email/i);
    expect(emailInput).toBeRequired();

    const generateBtn = screen.getByRole('button', { name: 'Generate' });
    expect(generateBtn).toBeDisabled();

    // Typing a plausibly-valid email enables the button.
    await userEvent.type(emailInput, 'alice@example.com');
    await waitFor(() => {
      expect(generateBtn).not.toBeDisabled();
    });
  });

  it('recipient column: shows the Recipient header + the email for each row', async () => {
    server.use(
      http.get('*/api/v1/acme/invitations', () =>
        HttpResponse.json([pendingInvitation, acceptedInvitation]),
      ),
    );

    renderWithProviders(<Invitations />, { route: ROUTE });

    // Column header appears.
    await waitFor(() => {
      expect(screen.getByRole('columnheader', { name: 'Recipient' })).toBeInTheDocument();
    });

    // Each row's recipient email renders.
    expect(screen.getByText('pending@example.com')).toBeInTheDocument();
    expect(screen.getByText('accepted@example.com')).toBeInTheDocument();
  });

  it('list rendering: pending, accepted, revoked rows show the correct status', async () => {
    server.use(
      http.get('*/api/v1/acme/invitations', () =>
        HttpResponse.json([pendingInvitation, acceptedInvitation, revokedInvitation]),
      ),
    );

    renderWithProviders(<Invitations />, { route: ROUTE });

    // Pending: CopyableInviteLink + expiry hint
    await waitFor(() => {
      expect(screen.getByText(/Expires in 5 days/)).toBeInTheDocument();
    });
    // Pending row exposes exactly one Revoke button
    expect(screen.getAllByRole('button', { name: 'Revoke' }).length).toBe(1);

    // Accepted: shows "Accepted by bob"
    expect(screen.getByText('Accepted by bob')).toBeInTheDocument();

    // Revoked row shows the revoked status pill.
    expect(screen.getByText('Revoked')).toBeInTheDocument();
  });

  it('revoke click: opens confirmation, confirm DELETEs the invitation', async () => {
    let deletedId: string | undefined;
    server.use(
      http.get('*/api/v1/acme/invitations', () =>
        HttpResponse.json([pendingInvitation]),
      ),
      http.delete('*/api/v1/acme/invitations/:id', ({ params }) => {
        deletedId = params.id as string;
        return new HttpResponse(null, { status: 204 });
      }),
    );

    renderWithProviders(<Invitations />, { route: ROUTE });

    // Wait for the pending row to render
    await waitFor(() => {
      expect(screen.getByText(/Expires in 5 days/)).toBeInTheDocument();
    });

    // Click Revoke on the pending row
    await userEvent.click(screen.getByRole('button', { name: 'Revoke' }));

    // Confirmation dialog appears
    await waitFor(() => {
      expect(screen.getByText('Revoke this invitation? The link will stop working immediately.')).toBeInTheDocument();
    });

    // The dialog renders its own Revoke confirm button — click the last
    // matching button since the table revoke button is also named "Revoke".
    const confirmButtons = screen.getAllByRole('button', { name: 'Revoke' });
    await userEvent.click(confirmButtons[confirmButtons.length - 1]);

    await waitFor(() => {
      expect(deletedId).toBe('inv-pending');
    });
  });
});
