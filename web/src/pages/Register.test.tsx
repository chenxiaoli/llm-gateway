import { describe, it, expect, vi } from 'vitest';
import { renderWithProviders } from '../test/render';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import Register from './Register';

const { mockNavigate, mockToastError } = vi.hoisted(() => ({
  mockNavigate: vi.fn(),
  mockToastError: vi.fn(),
}));

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom');
  return { ...actual, useNavigate: () => mockNavigate };
});
vi.mock('sonner', () => ({ toast: { error: mockToastError, success: vi.fn() } }));

describe('Register page', () => {
  it('renders registration form with username, email, password, and confirm password fields', () => {
    renderWithProviders(<Register />, { route: '/console/register' });

    expect(screen.getByText('Create Account')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Username')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Email')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Password')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Confirm Password')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Register' })).toBeInTheDocument();
  });

  it('shows confirm password validation error when passwords do not match', async () => {
    renderWithProviders(<Register />, { route: '/console/register' });

    await userEvent.type(screen.getByPlaceholderText('Username'), 'testuser');
    await userEvent.type(screen.getByPlaceholderText('Email'), 'test@example.com');
    await userEvent.type(screen.getByPlaceholderText('Password'), 'password123');
    await userEvent.type(screen.getByPlaceholderText('Confirm Password'), 'different');
    await userEvent.click(screen.getByRole('button', { name: 'Register' }));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Passwords do not match');
    });
  });

  it('shows registration disabled alert when allow_registration is false', async () => {
    server.use(
      http.get('*/api/v1/auth/config', () =>
        HttpResponse.json({ allow_registration: false }),
      ),
    );

    renderWithProviders(<Register />, { route: '/console/register' });

    await waitFor(() => {
      expect(screen.getByText('Registration is currently disabled')).toBeInTheDocument();
    });
  });

  it('disables submit button when registration is disabled', async () => {
    server.use(
      http.get('*/api/v1/auth/config', () =>
        HttpResponse.json({ allow_registration: false }),
      ),
    );

    renderWithProviders(<Register />, { route: '/console/register' });

    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Register' })).toBeDisabled();
    });
  });

  it('has a link to login page', () => {
    renderWithProviders(<Register />, { route: '/console/register' });

    expect(screen.getByText(/Already have an account/)).toBeInTheDocument();
  });

  it('shows backend error message when registration fails', async () => {
    server.use(
      http.post('*/api/v1/auth/register', () =>
        HttpResponse.json(
          { error: { message: 'Username already exists', type: 400 } },
          { status: 400 },
        ),
      ),
    );

    renderWithProviders(<Register />, { route: '/console/register' });

    await userEvent.type(screen.getByPlaceholderText('Username'), 'taken');
    await userEvent.type(screen.getByPlaceholderText('Email'), 'taken@example.com');
    await userEvent.type(screen.getByPlaceholderText('Password'), 'password123');
    await userEvent.type(screen.getByPlaceholderText('Confirm Password'), 'password123');
    await userEvent.click(screen.getByRole('button', { name: 'Register' }));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Username already exists');
    });
  });

  it('sends email in the register request body and redirects to /check-email', async () => {
    let capturedBody: Record<string, unknown> | undefined;
    server.use(
      http.post('*/api/v1/auth/register', async ({ request }) => {
        capturedBody = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json({
          token: 'test-jwt-token',
          refresh_token: 'test-refresh-jwt-token',
          user: { id: 'user-1', username: 'newuser', platform_role: null },
          current_org: null,
          orgs: [],
        });
      }),
    );

    renderWithProviders(<Register />, { route: '/register' });

    await userEvent.type(screen.getByPlaceholderText('Username'), 'newuser');
    await userEvent.type(screen.getByPlaceholderText('Email'), 'new@example.com');
    await userEvent.type(screen.getByPlaceholderText('Password'), 'password123');
    await userEvent.type(screen.getByPlaceholderText('Confirm Password'), 'password123');
    await userEvent.click(screen.getByRole('button', { name: 'Register' }));

    await waitFor(() => {
      expect(capturedBody).toBeDefined();
    });
    expect(capturedBody).toMatchObject({
      username: 'newuser',
      email: 'new@example.com',
      password: 'password123',
    });
    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith('/check-email', {
        state: { email: 'new@example.com' },
      });
    });
  });

  it('shows email_in_use toast when backend returns that error code', async () => {
    server.use(
      http.post('*/api/v1/auth/register', () =>
        HttpResponse.json(
          { error: { code: 'email_in_use', message: 'in use' } },
          { status: 400 },
        ),
      ),
    );

    renderWithProviders(<Register />, { route: '/register' });

    await userEvent.type(screen.getByPlaceholderText('Username'), 'dupe');
    await userEvent.type(screen.getByPlaceholderText('Email'), 'dupe@example.com');
    await userEvent.type(screen.getByPlaceholderText('Password'), 'password123');
    await userEvent.type(screen.getByPlaceholderText('Confirm Password'), 'password123');
    await userEvent.click(screen.getByRole('button', { name: 'Register' }));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('That email is already in use.');
    });
  });
});
