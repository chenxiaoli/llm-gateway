import { describe, it, expect } from 'vitest';
import { Toaster } from 'sonner';
import { renderWithProviders } from '../test/render';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import Groups from './Groups';
import type { Group } from '../types';

function renderGroups(route = '/admin/groups') {
  return renderWithProviders(
    <>
      <Groups />
      <Toaster />
    </>,
    { route },
  );
}

const mockGroups: Group[] = [
  {
    id: 'group-1',
    name: 'engineering',
    description: 'Engineering team',
    created_at: '2026-01-15T00:00:00Z',
    updated_at: '2026-01-15T00:00:00Z',
  },
  {
    id: 'group-2',
    name: 'production',
    description: null,
    created_at: '2026-02-20T00:00:00Z',
    updated_at: '2026-02-20T00:00:00Z',
  },
];

describe('Groups page', () => {
  it('renders empty state when no groups', async () => {
    server.use(
      http.get('*/api/v1/test-org/admin/groups', () => {
        return HttpResponse.json({ items: [], total: 0, page: 1, page_size: 20 });
      }),
    );

    renderGroups();

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'Groups' })).toBeInTheDocument();
    }, { timeout: 5000 });

    await waitFor(() => {
      expect(screen.getByText('No groups yet. Create your first group.')).toBeInTheDocument();
    }, { timeout: 5000 });
  }, 15000);

  it('renders groups in the table when groups exist', async () => {
    server.use(
      http.get('*/api/v1/test-org/admin/groups', () =>
        HttpResponse.json({ items: mockGroups, total: mockGroups.length, page: 1, page_size: 20 }),
      ),
    );

    renderGroups();

    await waitFor(() => {
      expect(screen.getByText('engineering')).toBeInTheDocument();
    }, { timeout: 5000 });

    expect(screen.getByText('Engineering team')).toBeInTheDocument();
    expect(screen.getByText('production')).toBeInTheDocument();
    expect(screen.getByText('Name')).toBeInTheDocument();
    expect(screen.getByText('Description')).toBeInTheDocument();
    expect(screen.getByText('Actions')).toBeInTheDocument();
  }, 15000);

  it('creates a group via the drawer', async () => {
    let capturedBody: unknown = null;

    server.use(
      http.get('*/api/v1/test-org/admin/groups', () => HttpResponse.json({ items: [], total: 0, page: 1, page_size: 20 })),
      http.post('*/api/v1/test-org/admin/groups', async ({ request }) => {
        capturedBody = await request.json();
        return HttpResponse.json({
          id: 'group-new',
          name: 'qa',
          description: 'Quality assurance',
          created_at: '2026-03-01T00:00:00Z',
          updated_at: '2026-03-01T00:00:00Z',
        });
      }),
    );

    renderGroups();

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'Groups' })).toBeInTheDocument();
    }, { timeout: 5000 });

    // Open drawer
    await userEvent.click(screen.getByRole('button', { name: /Add Group/i }));

    // Wait for drawer to open (title appears)
    await waitFor(() => {
      expect(screen.getByText('Create Group', { selector: 'h3' })).toBeInTheDocument();
    }, { timeout: 5000 });

    // Fill form
    const nameInput = screen.getByPlaceholderText('e.g. engineering');
    await userEvent.type(nameInput, 'qa');
    const descInput = screen.getByPlaceholderText("Brief description of this group's purpose");
    await userEvent.type(descInput, 'Quality assurance');

    // Submit
    await userEvent.click(screen.getByRole('button', { name: 'Create Group' }));

    await waitFor(() => {
      expect(capturedBody).toEqual({ name: 'qa', description: 'Quality assurance' });
    }, { timeout: 5000 });
  }, 20000);

  it('shows backend error on duplicate name', async () => {
    server.use(
      http.get('*/api/v1/test-org/admin/groups', () => HttpResponse.json({ items: [], total: 0, page: 1, page_size: 20 })),
      http.post('*/api/v1/test-org/admin/groups', () =>
        HttpResponse.json(
          { error: 'group already exists', code: 'GROUP_EXISTS' },
          { status: 409 },
        ),
      ),
    );

    renderGroups();

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: 'Groups' })).toBeInTheDocument();
    }, { timeout: 5000 });

    await userEvent.click(screen.getByRole('button', { name: /Add Group/i }));

    await waitFor(() => {
      expect(screen.getByText('Create Group', { selector: 'h3' })).toBeInTheDocument();
    }, { timeout: 5000 });

    const nameInput = screen.getByPlaceholderText('e.g. engineering');
    await userEvent.type(nameInput, 'engineering');

    await userEvent.click(screen.getByRole('button', { name: 'Create Group' }));

    // The 409 error should be displayed as a toast (via getErrorMessage fallback
    // since the response is not a plain string). The hook's onError handler
    // calls toast.error with the parsed error message.
    await waitFor(() => {
      // The toaster renders a toast element with role="status"
      const toasts = document.querySelectorAll('[data-sonner-toast]');
      expect(toasts.length).toBeGreaterThan(0);
    }, { timeout: 5000 });
  }, 20000);
});