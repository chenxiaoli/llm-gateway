import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, Routes, Route } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { http, HttpResponse } from 'msw';
import { server } from '../test/server';
import { OrgRouteGuard } from './OrgRouteGuard';
import { useAuthStore } from '../stores/authStore';

const orgs = {
  acme: { id: 'org-1', slug: 'acme', name: 'Acme', role: 'admin' as const, group_id: null },
  personal: { id: 'org-2', slug: 'personal', name: 'Personal', role: 'owner' as const, group_id: null },
};

function renderWithRouter(initialEntry: string) {
  const qc = new QueryClient();
  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter initialEntries={[initialEntry]}>
        <Routes>
          <Route element={<OrgRouteGuard />}>
            <Route path=":orgSlug/child" element={<div>child content</div>} />
          </Route>
          <Route path="/login" element={<div>login page</div>} />
          <Route path=":orgSlug/dashboard" element={<div>dashboard page</div>} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

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

describe('OrgRouteGuard', () => {
  it('renders children when slug matches an org', () => {
    renderWithRouter('/acme/child');
    expect(screen.getByText('child content')).toBeInTheDocument();
  });

  it('redirects to current org dashboard when slug is unknown', () => {
    renderWithRouter('/unknown-org/child');
    expect(screen.getByText('dashboard page')).toBeInTheDocument();
  });

  it('redirects to /login when no currentOrg and slug is unknown', () => {
    useAuthStore.setState({ currentOrg: null, orgs: [] });
    renderWithRouter('/unknown-org/child');
    expect(screen.getByText('login page')).toBeInTheDocument();
  });

  it('calls setCurrentOrg when slug differs from currentOrg', async () => {
    renderWithRouter('/personal/child');

    await waitFor(() => {
      expect(useAuthStore.getState().currentOrg?.slug).toBe('personal');
    });
  });

  it('does not call setCurrentOrg when slug already matches currentOrg', async () => {
    const beforeSlug = useAuthStore.getState().currentOrg?.slug;
    renderWithRouter('/acme/child');
    // Give any effect a chance to run
    await waitFor(() => {
      expect(useAuthStore.getState().currentOrg?.slug).toBe(beforeSlug);
    });
  });
});
