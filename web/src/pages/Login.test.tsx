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
    expect(screen.getByPlaceholderText('Username')).toBeInTheDocument();
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

    await userEvent.type(screen.getByPlaceholderText('Username'), 'admin');
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

    await userEvent.type(screen.getByPlaceholderText('Username'), 'wrong');
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

    await userEvent.type(screen.getByPlaceholderText('Username'), 'locked');
    await userEvent.type(screen.getByPlaceholderText('Password'), 'password');
    await userEvent.click(screen.getByRole('button', { name: 'Sign In' }));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Account locked');
    });
  });

  it('renders resend-verification panel when login returns email_not_verified', async () => {
    server.use(
      http.post('*/api/v1/auth/login', () =>
        HttpResponse.json(
          { error: { code: 'email_not_verified', message: 'verify your email' } },
          { status: 403 },
        ),
      ),
    );

    renderWithProviders(<Login />);

    await userEvent.type(screen.getByPlaceholderText('Username'), 'unverified');
    await userEvent.type(screen.getByPlaceholderText('Password'), 'password');
    await userEvent.click(screen.getByRole('button', { name: 'Sign In' }));

    // The inline Alert panel renders the "please verify" message and a
    // resend button — and crucially does NOT call navigate.
    await waitFor(() => {
      expect(screen.getByText('Please verify your email before logging in.')).toBeInTheDocument();
    });
    expect(screen.getByRole('button', { name: 'Resend verification email' })).toBeInTheDocument();
    expect(mockNavigate).not.toHaveBeenCalled();
  });

  it('resends verification email and toasts success', async () => {
    server.use(
      http.post('*/api/v1/auth/login', () =>
        HttpResponse.json(
          { error: { code: 'email_not_verified', message: 'verify your email' } },
          { status: 403 },
        ),
      ),
      http.post('*/api/v1/auth/resend-verification', () =>
        new HttpResponse(null, { status: 204 }),
      ),
    );

    renderWithProviders(<Login />);

    await userEvent.type(screen.getByPlaceholderText('Username'), 'unverified');
    await userEvent.type(screen.getByPlaceholderText('Password'), 'password');
    await userEvent.click(screen.getByRole('button', { name: 'Sign In' }));

    await waitFor(() => {
      expect(screen.getByPlaceholderText('Email')).toBeInTheDocument();
    });
    await userEvent.type(screen.getByPlaceholderText('Email'), 'me@example.com');
    await userEvent.click(screen.getByRole('button', { name: 'Resend verification email' }));

    await waitFor(() => {
      expect(mockToastSuccess).toHaveBeenCalledWith('Verification email sent.');
    });
  });
});
