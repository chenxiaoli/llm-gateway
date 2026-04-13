import { describe, it, expect, beforeEach, vi } from 'vitest';
import { renderWithProviders } from '../test/render';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { screen, waitFor, fireEvent } from '@testing-library/react';
import Register from './Register';

const navigate = vi.fn();
vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom');
  return { ...actual, useNavigate: () => navigate };
});

describe('Register page', () => {
  it('renders registration form with username, password, and confirm password fields', () => {
    renderWithProviders(<Register />, { route: '/console/register' });

    expect(screen.getByText('Create Account')).toBeInTheDocument();
    expect(screen.getByLabelText('Username')).toBeInTheDocument();
    expect(screen.getByLabelText('Password')).toBeInTheDocument();
    expect(screen.getByLabelText('Confirm Password')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Register' })).toBeInTheDocument();
  });

  it('shows password validation error for less than 6 characters', async () => {
    renderWithProviders(<Register />, { route: '/console/register' });

    const usernameInput = screen.getByLabelText('Username');
    const passwordInput = screen.getByLabelText('Password');
    const confirmInput = screen.getByLabelText('Confirm Password');

    fireEvent.change(usernameInput, { target: { value: 'testuser' } });
    fireEvent.change(passwordInput, { target: { value: 'abc' } });
    fireEvent.change(confirmInput, { target: { value: 'abc' } });
    fireEvent.click(screen.getByRole('button', { name: 'Register' }));

    await waitFor(() => {
      expect(screen.getByText('Password must be at least 6 characters')).toBeInTheDocument();
    });
  });

  it('shows confirm password validation error when passwords do not match', async () => {
    renderWithProviders(<Register />, { route: '/console/register' });

    const usernameInput = screen.getByLabelText('Username');
    const passwordInput = screen.getByLabelText('Password');
    const confirmInput = screen.getByLabelText('Confirm Password');

    fireEvent.change(usernameInput, { target: { value: 'testuser' } });
    fireEvent.change(passwordInput, { target: { value: 'password123' } });
    fireEvent.change(confirmInput, { target: { value: 'different' } });
    fireEvent.click(screen.getByRole('button', { name: 'Register' }));

    await waitFor(() => {
      expect(screen.getByText('Passwords do not match')).toBeInTheDocument();
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
