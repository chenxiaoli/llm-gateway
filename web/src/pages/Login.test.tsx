import { describe, it, expect, beforeEach, vi } from 'vitest';
import { renderWithProviders } from '../test/render';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import Login from './Login';
import { clearToken, setToken } from '../api/client';

const navigate = vi.fn();
vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom');
  return { ...actual, useNavigate: () => navigate };
});

beforeEach(() => {
  clearToken();
  navigate.mockClear();
});

describe('Login page', () => {
  it('renders login form', () => {
    renderWithProviders(<Login />);
    expect(screen.getByText('LLM Gateway')).toBeInTheDocument();
    expect(screen.getByLabelText('Username')).toBeInTheDocument();
    expect(screen.getByLabelText('Password')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Login' })).toBeInTheDocument();
  });

  it('shows registration link when allowed', async () => {
    renderWithProviders(<Login />);

    await waitFor(() => {
      expect(screen.getByText('Create an account')).toBeInTheDocument();
    });
  });

  it('navigates to dashboard on valid credentials', async () => {
    renderWithProviders(<Login />);

    await userEvent.type(screen.getByLabelText('Username'), 'admin');
    await userEvent.type(screen.getByLabelText('Password'), 'password');
    await userEvent.click(screen.getByRole('button', { name: 'Login' }));

    await waitFor(() => {
      expect(navigate).toHaveBeenCalledWith('/admin/dashboard');
    });
  });

  it('shows error on invalid credentials', async () => {
    server.use(
      http.post('*/api/v1/auth/login', () =>
        new HttpResponse(null, { status: 401 }),
      ),
    );

    renderWithProviders(<Login />);

    await userEvent.type(screen.getByLabelText('Username'), 'wrong');
    await userEvent.type(screen.getByLabelText('Password'), 'wrong');
    await userEvent.click(screen.getByRole('button', { name: 'Login' }));

    await waitFor(() => {
      expect(screen.getByText('Invalid username or password')).toBeInTheDocument();
    });
    expect(navigate).not.toHaveBeenCalled();
  });

  it('requires username and password fields', async () => {
    renderWithProviders(<Login />);

    await userEvent.click(screen.getByRole('button', { name: 'Login' }));

    await waitFor(() => {
      expect(screen.getByText('Enter your username')).toBeInTheDocument();
    });
    expect(screen.getByText('Enter your password')).toBeInTheDocument();
  });
});
