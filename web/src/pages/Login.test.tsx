import { describe, it, expect, beforeEach, vi } from 'vitest';
import { renderWithProviders } from '../test/render';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import Login from './Login';
import { clearToken } from '../api/client';

const { mockNavigate, mockToastError } = vi.hoisted(() => ({
  mockNavigate: vi.fn(),
  mockToastError: vi.fn(),
}));

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom');
  return { ...actual, useNavigate: () => mockNavigate };
});
vi.mock('sonner', () => ({ toast: { error: mockToastError, success: vi.fn() } }));

beforeEach(() => {
  clearToken();
  mockNavigate.mockClear();
  mockToastError.mockClear();
});

describe('Login page', () => {
  it('renders login form', () => {
    renderWithProviders(<Login />);
    expect(screen.getByText('LLM Gateway')).toBeInTheDocument();
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
      expect(mockNavigate).toHaveBeenCalledWith('/console/dashboard');
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
});
