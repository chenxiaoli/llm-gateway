import { describe, it, expect, vi } from 'vitest';
import { renderWithProviders } from '../test/render';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import CheckEmail from './CheckEmail';

const { mockToastSuccess } = vi.hoisted(() => ({
  mockToastSuccess: vi.fn(),
}));

vi.mock('sonner', () => ({
  toast: { success: mockToastSuccess, error: vi.fn() },
}));

describe('CheckEmail page', () => {
  it('renders the email from router state', () => {
    renderWithProviders(<CheckEmail />, {
      route: { pathname: '/check-email', state: { email: 'someone@example.com' } },
    });

    expect(screen.getByText('Check your email')).toBeInTheDocument();
    expect(screen.getByText(/someone@example.com/)).toBeInTheDocument();
  });

  it('redirects to /login when no email is provided in router state', () => {
    // No state at all → should redirect. We can't easily assert on the
    // Navigate render, but we can verify the page content (title) does NOT
    // render — meaning the component returned <Navigate> instead.
    renderWithProviders(<CheckEmail />, { route: '/check-email' });

    expect(screen.queryByText('Check your email')).not.toBeInTheDocument();
  });

  it('calls resendVerification when the resend button is clicked', async () => {
    let resendCalledWith: Record<string, unknown> | undefined;
    server.use(
      http.post('*/api/v1/auth/resend-verification', async ({ request }) => {
        resendCalledWith = (await request.json()) as Record<string, unknown>;
        return new HttpResponse(null, { status: 204 });
      }),
    );

    renderWithProviders(<CheckEmail />, {
      route: { pathname: '/check-email', state: { email: 'again@example.com' } },
    });

    await userEvent.click(screen.getByRole('button', { name: 'Resend email' }));

    await waitFor(() => {
      expect(resendCalledWith).toEqual({ email: 'again@example.com' });
    });
    await waitFor(() => {
      expect(mockToastSuccess).toHaveBeenCalledWith('Verification email resent.');
    });
  });
});
