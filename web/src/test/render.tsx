import { render } from '@testing-library/react';
import type { ReactElement, ReactNode } from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { MemoryRouter, type InitialEntry } from 'react-router-dom';
import { useAuthStore } from '../stores/authStore';
import '../i18n';

function createTestQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0 },
      mutations: { retry: false },
    },
  });
}

export interface RenderOptions {
  /** Path string OR a router state-carrying entry (e.g. { pathname, state }). */
  route?: InitialEntry;
  queryClient?: QueryClient;
}

export function renderWithProviders(
  ui: ReactElement,
  options?: RenderOptions,
) {
  const queryClient = options?.queryClient ?? createTestQueryClient();
  const route = options?.route ?? '/';

  function Wrapper({ children }: { children: ReactNode }) {
    return (
      <QueryClientProvider client={queryClient}>
        <MemoryRouter initialEntries={[route]}>
          {children}
        </MemoryRouter>
      </QueryClientProvider>
    );
  }

  return { ...render(ui, { wrapper: Wrapper }), queryClient };
}

export { render };
