import { describe, it, expect } from 'vitest';
import { renderWithProviders } from '../test/render';
import { screen, waitFor } from '@testing-library/react';
import AccountBalance from './AccountBalance';

describe('AccountBalance page', () => {
  it('renders Account Balance heading', async () => {
    renderWithProviders(<AccountBalance />, { route: '/admin/users/test-user-id/balance' });

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'Account Balance' })).toBeInTheDocument();
    }, { timeout: 3000 });
  });

  it('renders Recharge and Adjust buttons', async () => {
    renderWithProviders(<AccountBalance />, { route: '/admin/users/test-user-id/balance' });

    await waitFor(() => {
      expect(screen.getByRole('button', { name: 'Recharge' })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: 'Adjust' })).toBeInTheDocument();
    }, { timeout: 3000 });
  });
});
