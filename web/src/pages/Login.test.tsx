import { describe, it, expect, beforeEach, vi } from 'vitest';
import { renderWithProviders } from '../test/render';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import Login from './Login';
import { clearToken } from '../api/client';

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
    expect(screen.getByLabelText('Admin Token')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Login' })).toBeInTheDocument();
  });

  it('navigates to dashboard on valid token', async () => {
    server.use(
      http.get('*/api/v1/keys', () => HttpResponse.json([])),
    );

    renderWithProviders(<Login />);

    const input = screen.getByLabelText('Admin Token');
    await userEvent.type(input, 'valid-token');
    await userEvent.click(screen.getByRole('button', { name: 'Login' }));

    await waitFor(() => {
      expect(navigate).toHaveBeenCalledWith('/admin/');
    });
  });

  it('shows error on invalid token', async () => {
    server.use(
      http.get('*/api/v1/keys', () =>
        new HttpResponse(null, { status: 401 }),
      ),
    );

    renderWithProviders(<Login />);

    const input = screen.getByLabelText('Admin Token');
    await userEvent.type(input, 'wrong-token');
    await userEvent.click(screen.getByRole('button', { name: 'Login' }));

    await waitFor(() => {
      expect(screen.getByText('Invalid admin token')).toBeInTheDocument();
    });
    expect(navigate).not.toHaveBeenCalled();
  });

  it('requires token field', async () => {
    renderWithProviders(<Login />);

    await userEvent.click(screen.getByRole('button', { name: 'Login' }));

    expect(await screen.findByText(/required/)).toBeInTheDocument();
  });
});
