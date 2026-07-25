import { describe, it, expect, beforeEach, vi } from 'vitest';
import { renderWithProviders } from '../test/render';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import Login from './Login';
import { clearToken } from '../api/client';

const { mockNavigate, mockToastError, mockToastSuccess } = vi.hoisted(() => ({
  mockNavigate: vi.fn(),
  mockToastError: vi.fn(),
  mockToastSuccess: vi.fn(),
}));

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom');
  return { ...actual, useNavigate: () => mockNavigate };
});
vi.mock('sonner', () => ({
  toast: { error: mockToastError, success: mockToastSuccess },
}));

beforeEach(() => {
  clearToken();
  mockNavigate.mockClear();
  mockToastError.mockClear();
  mockToastSuccess.mockClear();
});

describe('Login page', () => {
  it('renders login form', () => {
    renderWithProviders(<Login />);
    expect(screen.getByText('TokenVis')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Username or email')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Password')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Sign In' })).toBeInTheDocument();
  });

  it('shows registration link when allowed', async () => {
    renderWithProviders(<Login />);

    await waitFor(() => {
      expect(screen.getByText('Create one')).toBeInTheDocument();
    });
  });

  it('navigates to dashboard on valid credentials', async () => {
    renderWithProviders(<Login />);

    await userEvent.type(screen.getByPlaceholderText('Username or email'), 'admin');
    await userEvent.type(screen.getByPlaceholderText('Password'), 'password');
    await userEvent.click(screen.getByRole('button', { name: 'Sign In' }));

    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith('/default/dashboard');
    });
  });

  it('shows error on invalid credentials', async () => {
    server.use(
      http.post('*/api/v1/auth/login', () =>
        new HttpResponse(null, { status: 401 }),
      ),
    );

    renderWithProviders(<Login />);

    await userEvent.type(screen.getByPlaceholderText('Username or email'), 'wrong');
    await userEvent.type(screen.getByPlaceholderText('Password'), 'wrong');
    await userEvent.click(screen.getByRole('button', { name: 'Sign In' }));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Invalid username or password');
    });
    expect(mockNavigate).not.toHaveBeenCalled();
  });

  it('does not submit with empty fields', async () => {
    renderWithProviders(<Login />);

    await userEvent.click(screen.getByRole('button', { name: 'Sign In' }));

    expect(mockNavigate).not.toHaveBeenCalled();
  });

  it('shows backend error message when login fails with body', async () => {
    server.use(
      http.post('*/api/v1/auth/login', () =>
        HttpResponse.json(
          { error: { message: 'Account locked', type: 403 } },
          { status: 403 },
        ),
      ),
    );

    renderWithProviders(<Login />);

    await userEvent.type(screen.getByPlaceholderText('Username or email'), 'locked');
    await userEvent.type(screen.getByPlaceholderText('Password'), 'password');
    await userEvent.click(screen.getByRole('button', { name: 'Sign In' }));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Account locked');
    });
  });

  it('renders fresh-sent verification panel when login returns email_not_verified', async () => {
    server.use(
      http.post('*/api/v1/auth/login', () =>
        HttpResponse.json(
          { error: { code: 'email_not_verified', message: 'verify your email' } },
          { status: 403 },
        ),
      ),
    );

    renderWithProviders(<Login />);

    await userEvent.type(screen.getByPlaceholderText('Username or email'), 'unverified');
    await userEvent.type(screen.getByPlaceholderText('Password'), 'password');
    await userEvent.click(screen.getByRole('button', { name: 'Sign In' }));

    // The backend dispatches a fresh verification email as part of the
    // email_not_verified response, so the panel only needs to surface the
    // "we just sent" message + toast — no email input, no resend button.
    await waitFor(() => {
      expect(
        screen.getByText(
          'We just sent a fresh verification link to your email. Check your inbox and click the link to verify.',
        ),
      ).toBeInTheDocument();
    });
    expect(mockToastSuccess).toHaveBeenCalledWith('Verification email sent.');
    // No email input, no resend button — the path is one-click.
    expect(screen.queryByPlaceholderText('Email')).not.toBeInTheDocument();
    expect(
      screen.queryByRole('button', { name: 'Resend verification email' }),
    ).not.toBeInTheDocument();
    expect(mockNavigate).not.toHaveBeenCalled();
  });
});
