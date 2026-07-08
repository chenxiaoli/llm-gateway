import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { http, HttpResponse } from 'msw';
import { server } from '../test/server';
import { OrgSwitcher } from './OrgSwitcher';
import { useAuthStore } from '../stores/authStore';
import { queryClient } from '../lib/queryClient';

const orgs = {
  acme: { id: 'org-1', slug: 'acme', name: 'Acme', role: 'admin' as const, group_id: null },
  personal: { id: 'org-2', slug: 'personal', name: 'Personal', role: 'owner' as const, group_id: null },
};

beforeEach(() => {
  server.use(
    http.post('*/api/v1/me/current-org', async ({ request }) => {
      const body = (await request.json()) as { org_slug: string };
      const target = body.org_slug === 'personal' ? orgs.personal : orgs.acme;
      return HttpResponse.json({
        token: 'new-token',
        refresh_token: 'new-refresh',
        user: { id: 'u', username: 'u', platform_role: null },
        current_org: target,
        orgs: [orgs.acme, orgs.personal],
      });
    }),
  );
  useAuthStore.setState({
    currentOrg: orgs.acme,
    orgs: [orgs.acme, orgs.personal],
    user: { id: 'u', username: 'u', platform_role: null },
  });
});

describe('OrgSwitcher', () => {
  function renderWithProviders(initialRoute = '/acme/dashboard') {
    const qc = new QueryClient();
    const result = render(
      <QueryClientProvider client={qc}>
        <MemoryRouter initialEntries={[initialRoute]}>
          <OrgSwitcher />
        </MemoryRouter>
      </QueryClientProvider>,
    );
    return { qc, ...result };
  }

  it('shows the current org name', () => {
    renderWithProviders();
    expect(screen.getByText('Acme')).toBeInTheDocument();
  });

  it('renders nothing when currentOrg is null', () => {
    useAuthStore.setState({ currentOrg: null });
    const { container } = renderWithProviders();
    expect(container.firstChild).toBeNull();
  });

  it('lists all orgs when opened', () => {
    renderWithProviders();
    fireEvent.click(screen.getByText('Acme'));
    expect(screen.getByText('Personal')).toBeInTheDocument();
    expect(screen.getByText(/Create org/)).toBeInTheDocument();
  });

  it('switches org on click — calls switchOrg, clears cache', async () => {
    // Production code calls the singleton queryClient from lib/queryClient,
    // not the per-test QueryClient used for the provider — spy on the singleton.
    const clearSpy = vi.spyOn(queryClient, 'clear');

    renderWithProviders();

    fireEvent.click(screen.getByText('Acme'));
    fireEvent.click(screen.getByText('Personal'));

    await waitFor(() => {
      expect(clearSpy).toHaveBeenCalled();
    });

    // currentOrg updated in store, tokens rotated
    expect(useAuthStore.getState().currentOrg?.slug).toBe('personal');
    expect(localStorage.getItem('llm_gateway_admin_token')).toBe('new-token');
    expect(localStorage.getItem('llm_gateway_refresh_token')).toBe('new-refresh');

    clearSpy.mockRestore();
  });

  it('closes dropdown on outside click', () => {
    renderWithProviders();
    fireEvent.click(screen.getByText('Acme'));
    expect(screen.getByText('Personal')).toBeInTheDocument();
    fireEvent.mouseDown(document.body);
    expect(screen.queryByText('Personal')).not.toBeInTheDocument();
  });
});
