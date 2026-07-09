import { describe, it, expect, vi, beforeEach } from 'vitest';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithProviders } from '../test/render';
import { useAuthStore } from '../stores/authStore';
import type { User } from '../types';
import { AddEmailModal } from './AddEmailModal';

const { mockToastSuccess } = vi.hoisted(() => ({
  mockToastSuccess: vi.fn(),
}));

vi.mock('sonner', () => ({
  toast: { success: mockToastSuccess, error: vi.fn() },
}));

const baseUser: User = {
  id: 'u1',
  username: 'alice',
  platform_role: null,
  email: null,
  email_verified_at: null,
};

describe('AddEmailModal', () => {
  beforeEach(() => {
    useAuthStore.setState({ user: baseUser });
  });

  it('renders the form when open=true', () => {
    renderWithProviders(<AddEmailModal open onClose={() => {}} />);
    expect(screen.getByText('Add your email')).toBeInTheDocument();
    expect(screen.getByLabelText('Email')).toBeInTheDocument();
  });

  it('on submit success: calls API, toasts, closes, updates store email', async () => {
    let submittedBody: Record<string, unknown> | undefined;
    server.use(
      http.post('*/api/v1/auth/me/email', async ({ request }) => {
        submittedBody = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json({
          id: 'u1',
          username: 'alice',
          platform_role: null,
          current_org: null,
          orgs: [],
          allow_registration: true,
          impersonating: false,
          email: 'alice@example.com',
          email_verified_at: null,
          requires_email_verification: false,
        });
      }),
    );
    const onClose = vi.fn();

    renderWithProviders(<AddEmailModal open onClose={onClose} />);
    const input = screen.getByLabelText('Email');
    await userEvent.type(input, 'alice@example.com');
    await userEvent.click(screen.getByRole('button', { name: 'Send verification' }));

    await waitFor(() => {
      expect(submittedBody).toEqual({ email: 'alice@example.com' });
    });
    await waitFor(() => {
      expect(mockToastSuccess).toHaveBeenCalledWith('Verification email sent — check your inbox.');
    });
    await waitFor(() => {
      expect(onClose).toHaveBeenCalled();
    });
    expect(useAuthStore.getState().user?.email).toBe('alice@example.com');
  });

  it('on 409 email_in_use: shows inline error, modal stays open', async () => {
    server.use(
      http.post('*/api/v1/auth/me/email', () =>
        HttpResponse.json(
          { error: { message: 'Email is already in use', type: 409, code: 'email_in_use' } },
          { status: 409 },
        ),
      ),
    );
    const onClose = vi.fn();

    renderWithProviders(<AddEmailModal open onClose={onClose} />);
    await userEvent.type(screen.getByLabelText('Email'), 'taken@example.com');
    await userEvent.click(screen.getByRole('button', { name: 'Send verification' }));

    await waitFor(() => {
      expect(screen.getByText('That email is already in use.')).toBeInTheDocument();
    });
    expect(onClose).not.toHaveBeenCalled();
    expect(useAuthStore.getState().user?.email).toBeNull();
  });

  it('on generic 500: shows generic error', async () => {
    server.use(
      http.post('*/api/v1/auth/me/email', () =>
        HttpResponse.json(
          { error: { message: 'database is down', type: 500 } },
          { status: 500 },
        ),
      ),
    );

    renderWithProviders(<AddEmailModal open onClose={() => {}} />);
    await userEvent.type(screen.getByLabelText('Email'), 'alice@example.com');
    await userEvent.click(screen.getByRole('button', { name: 'Send verification' }));

    await waitFor(() => {
      expect(screen.getByText('database is down')).toBeInTheDocument();
    });
  });
});
