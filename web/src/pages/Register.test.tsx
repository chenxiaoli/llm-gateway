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
  it('renders registration form with username, password, and confirm password fields', () => {
    renderWithProviders(<Register />, { route: '/console/register' });

    expect(screen.getByText('Create Account')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Username')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Password')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Confirm password')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Register' })).toBeInTheDocument();
  });

  it('shows confirm password validation error when passwords do not match', async () => {
    renderWithProviders(<Register />, { route: '/console/register' });

    await userEvent.type(screen.getByPlaceholderText('Username'), 'testuser');
    await userEvent.type(screen.getByPlaceholderText('Password'), 'password123');
    await userEvent.type(screen.getByPlaceholderText('Confirm password'), 'different');
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
});
