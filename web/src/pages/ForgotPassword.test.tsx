import { describe, it, expect, vi } from 'vitest';
import { renderWithProviders } from '../test/render';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import ForgotPassword from './ForgotPassword';

const { mockToastError } = vi.hoisted(() => ({
  mockToastError: vi.fn(),
}));

vi.mock('sonner', () => ({
  toast: { success: vi.fn(), error: mockToastError },
}));

describe('ForgotPassword page', () => {
  it('renders the form (title, email input, submit button)', () => {
    renderWithProviders(<ForgotPassword />, { route: '/forgot-password' });

    expect(screen.getByText('Reset your password')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('you@example.com')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Send reset link' })).toBeInTheDocument();
  });

  it('shows the success panel with the email echoed back after a successful submit', async () => {
    let calledWith: Record<string, unknown> | undefined;
    server.use(
      http.post('*/api/v1/auth/password-reset/request', async ({ request }) => {
        calledWith = (await request.json()) as Record<string, unknown>;
        return new HttpResponse(null, { status: 204 });
      }),
    );

    renderWithProviders(<ForgotPassword />, { route: '/forgot-password' });

    await userEvent.type(screen.getByPlaceholderText('you@example.com'), 'someone@example.com');
    await userEvent.click(screen.getByRole('button', { name: 'Send reset link' }));

    await waitFor(() => {
      expect(calledWith).toEqual({ email: 'someone@example.com' });
    });
    await waitFor(() => {
      expect(screen.getByText('Check your email')).toBeInTheDocument();
    });
    expect(screen.getByText(/someone@example.com/)).toBeInTheDocument();
  });

  it('still shows the success panel when the backend errors (never leaks)', async () => {
    // Backend is intentionally always-204, but we test the defensive path —
    // if it does error, the UI must NOT surface a distinct error message that
    // would reveal whether the email is registered.
    server.use(
      http.post('*/api/v1/auth/password-reset/request', () =>
        HttpResponse.json(
          { error: { code: 'internal', message: 'boom' } },
          { status: 500 },
        ),
      ),
    );

    renderWithProviders(<ForgotPassword />, { route: '/forgot-password' });

    await userEvent.type(screen.getByPlaceholderText('you@example.com'), 'ghost@example.com');
    await userEvent.click(screen.getByRole('button', { name: 'Send reset link' }));

    await waitFor(() => {
      expect(screen.getByText('Check your email')).toBeInTheDocument();
    });
    expect(screen.getByText(/ghost@example.com/)).toBeInTheDocument();
    // No error toast should fire on this path.
    expect(mockToastError).not.toHaveBeenCalled();
  });

  it('renders a Back to login link', () => {
    renderWithProviders(<ForgotPassword />, { route: '/forgot-password' });

    const loginLinks = screen.getAllByRole('link', { name: 'Back to login' });
    expect(loginLinks.length).toBeGreaterThan(0);
  });
});
