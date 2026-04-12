import { setupServer } from 'msw/node';
import { http, HttpResponse } from 'msw';

export const server = setupServer(
  http.get('*/api/v1/keys', () => {
    return HttpResponse.json([]);
  }),
  http.get('*/api/v1/providers', () => {
    return HttpResponse.json([]);
  }),
  http.get('*/api/v1/usage', () => {
    return HttpResponse.json([]);
  }),
  http.get('*/api/v1/logs', () => {
    return HttpResponse.json([]);
  }),
);
