import { describe, it, expect } from 'vitest';
import { renderWithProviders } from '../test/render';
import { screen, waitFor } from '@testing-library/react';
import Users from './Users';

describe('Users page', () => {
  it('renders users table', async () => {
    renderWithProviders(<Users />, { route: '/admin/users' });

    await waitFor(() => {
      expect(screen.getByRole('table')).toBeInTheDocument();
    }, { timeout: 5000 });
  });

  it('shows Users title', async () => {
    renderWithProviders(<Users />, { route: '/admin/users' });

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'Users' })).toBeInTheDocument();
    }, { timeout: 5000 });
  });

  it('has columns: Username, Role, Status, Created, Actions', async () => {
    renderWithProviders(<Users />, { route: '/admin/users' });

    await waitFor(() => {
      expect(screen.getByText('Username')).toBeInTheDocument();
      expect(screen.getByText('Role')).toBeInTheDocument();
      expect(screen.getByText('Status')).toBeInTheDocument();
      expect(screen.getByText('Created')).toBeInTheDocument();
      expect(screen.getByText('Actions')).toBeInTheDocument();
    }, { timeout: 5000 });
  });
});
