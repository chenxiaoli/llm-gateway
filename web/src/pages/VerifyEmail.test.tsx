import { describe, it, expect, vi, beforeAll, afterAll } from 'vitest';
import { renderWithProviders } from '../test/render';
import { server } from '../test/server';
import { http, HttpResponse } from 'msw';
import { screen, waitFor } from '@testing-library/react';
import { Routes, Route } from 'react-router-dom';
import VerifyEmail from './VerifyEmail';

const { mockNavigate } = vi.hoisted(() => ({
  mockNavigate: vi.fn(),
}));

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual('react-router-dom');
  return { ...actual, useNavigate: () => mockNavigate };
});

/**
 * VerifyEmail reads :token via useParams, so we have to mount it inside a
 * matching <Route>. The render helper only sets the URL; it doesn't define
 * routes. Wrap the component in a <Routes> with the same path the router
 * initial entry uses.
 */
function renderVerifyEmailAt(path: string) {
  return renderWithProviders(
    <Routes>
      <Route path="/verify-email/:token" element={<VerifyEmail />} />
    </Routes>,
    { route: path },
  );
}

beforeAll(() => {
  // useNavigate is mocked at module level; reset between tests via test-specific
  // setup. msw handlers installed per-test below override the defaults.
});
afterAll(() => {
  server.resetHandlers();
});

describe('VerifyEmail page', () => {
  it('shows verified panel when the backend returns 204', async () => {
    server.use(
      http.post('*/api/v1/auth/verify-email', () =>
        new HttpResponse(null, { status: 204 }),
      ),
    );

    renderVerifyEmailAt('/verify-email/good-token');

    await waitFor(() => {
      expect(screen.getByText('Email verified')).toBeInTheDocument();
    });
    expect(screen.getByText('Continue to login')).toBeInTheDocument();
  });

  it('shows expired panel when the backend returns 410 with verification_expired', async () => {
    server.use(
      http.post('*/api/v1/auth/verify-email', () =>
        HttpResponse.json(
          { error: { code: 'verification_expired', message: 'link expired' } },
          { status: 410 },
        ),
      ),
    );

    renderVerifyEmailAt('/verify-email/expired-token');

    await waitFor(() => {
      expect(screen.getByText('Link expired or invalid')).toBeInTheDocument();
    });
  });

  it('shows error panel on unexpected server error', async () => {
    server.use(
      http.post('*/api/v1/auth/verify-email', () =>
        HttpResponse.json(
          { error: { code: 'internal', message: 'boom' } },
          { status: 500 },
        ),
      ),
    );

    renderVerifyEmailAt('/verify-email/broken');

    await waitFor(() => {
      expect(screen.getByText('Something went wrong')).toBeInTheDocument();
    });
    expect(screen.getByRole('button', { name: 'Try again' })).toBeInTheDocument();
  });
});
