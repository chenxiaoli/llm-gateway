import { setupServer } from 'msw/node';
import { http, HttpResponse } from 'msw';

export const server = setupServer(
  http.get('*/api/v1/auth/config', () => {
    return HttpResponse.json({ allow_registration: true });
  }),
  http.get('*/api/v1/auth/me', () => {
    return HttpResponse.json({
      id: 'user-1',
      username: 'admin',
      role: 'admin',
      allow_registration: true,
    });
  }),
  http.post('*/api/v1/auth/login', () => {
    return HttpResponse.json({
      token: 'test-jwt-token',
      user: { id: 'user-1', username: 'admin', role: 'admin' },
    });
  }),
  http.post('*/api/v1/auth/register', () => {
    return HttpResponse.json({
      token: 'test-jwt-token',
      user: { id: 'user-1', username: 'admin', role: 'admin' },
    });
  }),
  http.get('*/api/v1/keys', () => {
    return HttpResponse.json([]);
  }),
  http.get('*/api/v1/providers', () => {
    return HttpResponse.json([]);
  }),
  http.get('*/api/v1/users', () => {
    return HttpResponse.json([]);
  }),
  http.get('*/api/v1/settings', () => {
    return HttpResponse.json({ allow_registration: true });
  }),
  http.get('*/api/v1/usage', () => {
    return HttpResponse.json([]);
  }),
  http.get('*/api/v1/logs', () => {
    return HttpResponse.json([]);
  }),
);
