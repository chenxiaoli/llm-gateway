import { describe, it, expect, beforeEach } from 'vitest';
import { Toaster } from 'sonner';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { renderWithProviders } from '../test/render';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import Profile from './Profile';
import { useAuthStore } from '../stores/authStore';

describe('Profile page', () => {
  beforeEach(() => {
    // Seed an authenticated user with no nickname. Page-level tests in this
    // repo don't seed localStorage tokens (MSW handlers don't enforce auth);
    // setting the auth-store user is what makes the page render content.
    useAuthStore.setState({
      user: {
        id: 'u1',
        username: null,
        email: 'me@x.com',
        nickname: null,
        platform_role: null,
        email_verified_at: null,
      },
      currentOrg: {
        id: 'o1',
        slug: 'o1',
        name: 'Org',
        role: 'owner',
        group_id: null,
      },
    });
  });

  it('renders current nickname/username/email', async () => {
    renderWithProviders(<Profile />);
    // Empty nickname input — findByDisplayValue('') is ambiguous (any empty
    // input matches), so look the input up by placeholder and assert value.
    const input = await screen.findByPlaceholderText('Your nickname');
    expect(input).toHaveValue('');
    expect(screen.getByText('me@x.com')).toBeInTheDocument();
  });

  it('saves nickname and shows success toast', async () => {
    server.use(
      http.post('*/api/v1/auth/me/nickname', async ({ request }) => {
        const body = (await request.json()) as { nickname: string };
        return HttpResponse.json({
          id: 'u1',
          username: null,
          platform_role: null,
          nickname: body.nickname,
          current_org: null,
          orgs: [],
          allow_registration: true,
          impersonating: false,
        });
      }),
    );

    renderWithProviders(
      <>
        <Profile />
        <Toaster />
      </>,
    );
    const input = await screen.findByPlaceholderText('Your nickname');
    await userEvent.type(input, 'Alice');
    await userEvent.click(screen.getByRole('button', { name: /save/i }));

    await waitFor(() => {
      expect(screen.getByText('Nickname updated')).toBeInTheDocument();
    });
  });

  it('rejects over-length input client-side', async () => {
    renderWithProviders(<Profile />);
    const input = await screen.findByPlaceholderText('Your nickname');
    await userEvent.type(input, 'x'.repeat(33));
    await userEvent.click(screen.getByRole('button', { name: /save/i }));

    expect(await screen.findByText(/must be 1–32 characters/i)).toBeInTheDocument();
  });
});
