import '@testing-library/jest-dom/vitest';
import { server } from './server';
import { useAuthStore } from '../stores/authStore';
import type { OrgSummary } from '../types';

Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  }),
});

beforeAll(() => server.listen({ onUnhandledRequest: 'error' }));
beforeEach(() => {
  // Seed a current org so orgPrefix() resolves to /api/v1/test-org in all tests.
  // Individual tests can override via useAuthStore.setState({ currentOrg: ... }).
  const org: OrgSummary = {
    id: 'org-1',
    slug: 'test-org',
    name: 'Test Org',
    role: 'admin',
    group_id: null,
  };
  useAuthStore.setState({ currentOrg: org });
});
afterEach(() => server.resetHandlers());
afterAll(() => server.close());
