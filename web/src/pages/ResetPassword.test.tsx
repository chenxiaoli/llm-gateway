import { describe, it, expect, vi } from 'vitest';
import { renderWithProviders } from '../test/render';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Routes, Route } from 'react-router-dom';
import ResetPassword from './ResetPassword';

const { mockToastSuccess, mockToastError, mockNavigate } = vi.hoisted(() => ({
  mockToastSuccess: vi.fn(),
  mockToastError: vi.fn(),
  mockNavigate: vi.fn(),
}));

vi.mock('sonner', () => ({
  toast: { success: mockToastSuccess, error: mockToastError },
}));

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom');
  return { ...actual, useNavigate: () => mockNavigate };
});

/**
 * ResetPassword reads :token via useParams, so mount it inside a matching
 * <Route> — mirrors the VerifyEmail.test.tsx pattern.
 */
function renderResetPasswordAt(path: string) {
  return renderWithProviders(
    <Routes>
      <Route path="/reset-password/:token" element={<ResetPassword />} />
    </Routes>,
    { route: path },
  );
}

describe('ResetPassword page', () => {
  it('renders the new-password form when the preview returns valid', async () => {
    server.use(
      http.get('*/api/v1/auth/password-reset/preview', () =>
        HttpResponse.json({ valid: true, expires_at: '2026-07-09T12:00:00Z' }),
      ),
    );

    renderResetPasswordAt('/reset-password/good-token');

    await waitFor(() => {
      expect(screen.getByText('Set a new password')).toBeInTheDocument();
    });
    expect(screen.getByPlaceholderText('New password')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Confirm new password')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Update password' })).toBeInTheDocument();
  });

  it('renders the expired panel when the preview returns valid:false', async () => {
    server.use(
      http.get('*/api/v1/auth/password-reset/preview', () =>
        HttpResponse.json({ valid: false, expires_at: null }),
      ),
    );

    renderResetPasswordAt('/reset-password/stale-token');

    await waitFor(() => {
      expect(screen.getByText('Link expired or invalid')).toBeInTheDocument();
    });
    expect(screen.getByText('Request new link')).toBeInTheDocument();
  });

  it('renders the error panel on a network/preview error', async () => {
    server.use(
      http.get('*/api/v1/auth/password-reset/preview', () =>
        HttpResponse.json(
          { error: { code: 'internal', message: 'boom' } },
          { status: 500 },
        ),
      ),
    );

    renderResetPasswordAt('/reset-password/broken');

    await waitFor(() => {
      expect(screen.getByText('Something went wrong')).toBeInTheDocument();
    });
    expect(screen.getByRole('button', { name: 'Try again' })).toBeInTheDocument();
  });

  it('rejects mismatched passwords with a toast and does not call confirm', async () => {
    let confirmCalled = false;
    server.use(
      http.get('*/api/v1/auth/password-reset/preview', () =>
        HttpResponse.json({ valid: true, expires_at: '2026-07-09T12:00:00Z' }),
      ),
      http.post('*/api/v1/auth/password-reset/confirm', () => {
        confirmCalled = true;
        return new HttpResponse(null, { status: 204 });
      }),
    );

    renderResetPasswordAt('/reset-password/good-token');

    await waitFor(() => {
      expect(screen.getByPlaceholderText('New password')).toBeInTheDocument();
    });

    await userEvent.type(screen.getByPlaceholderText('New password'), 'password123');
    await userEvent.type(screen.getByPlaceholderText('Confirm new password'), 'password456');
    await userEvent.click(screen.getByRole('button', { name: 'Update password' }));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Passwords do not match');
    });
    // Give the event loop a tick to ensure no confirm call slipped through.
    await new Promise((r) => setTimeout(r, 50));
    expect(confirmCalled).toBe(false);
  });

  it('shows the success panel on a valid confirm submission', async () => {
    server.use(
      http.get('*/api/v1/auth/password-reset/preview', () =>
        HttpResponse.json({ valid: true, expires_at: '2026-07-09T12:00:00Z' }),
      ),
      http.post('*/api/v1/auth/password-reset/confirm', () =>
        new HttpResponse(null, { status: 204 }),
      ),
    );

    renderResetPasswordAt('/reset-password/good-token');

    await waitFor(() => {
      expect(screen.getByPlaceholderText('New password')).toBeInTheDocument();
    });

    await userEvent.type(screen.getByPlaceholderText('New password'), 'newpassword');
    await userEvent.type(screen.getByPlaceholderText('Confirm new password'), 'newpassword');
    await userEvent.click(screen.getByRole('button', { name: 'Update password' }));

    await waitFor(() => {
      expect(screen.getByText('Password updated')).toBeInTheDocument();
    });
    await waitFor(() => {
      expect(mockToastSuccess).toHaveBeenCalledWith('Password updated');
    });
  });
});
