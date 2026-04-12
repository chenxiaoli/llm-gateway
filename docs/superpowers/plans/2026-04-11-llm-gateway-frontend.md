# LLM Gateway Frontend Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a React SPA management dashboard for the LLM Gateway, providing key/provider/model CRUD, usage analytics, and audit log browsing.

**Architecture:** Single-page app with React Router for client-side routing. React Query manages all server state (fetching, caching, mutations). Ant Design provides the UI component library. The app is served as static files by the Rust gateway binary at `/admin/`.

**Tech Stack:** React 18, TypeScript, Vite, React Router v6, React Query v5, Ant Design 5, Recharts, Axios

---

## Backend API Contract Reference

### Auth
All management endpoints require: `Authorization: Bearer <admin_token>`

### API Keys
| Method | Path | Request Body | Response |
|---|---|---|---|
| POST | `/api/v1/keys` | `{ name, rate_limit?, budget_monthly? }` | `{ id, name, key, rate_limit?, budget_monthly?, enabled, created_at }` |
| GET | `/api/v1/keys` | — | `ApiKey[]` |
| GET | `/api/v1/keys/:id` | — | `ApiKey` |
| PATCH | `/api/v1/keys/:id` | `{ name?, rate_limit?, budget_monthly?, enabled? }` (use `null` to clear) | `ApiKey` |
| DELETE | `/api/v1/keys/:id` | — | 204 No Content |

`ApiKey` = `{ id, name, key_hash, rate_limit?, budget_monthly?, enabled, created_at, updated_at }`

### Providers
| Method | Path | Request Body | Response |
|---|---|---|---|
| POST | `/api/v1/providers` | `{ name, api_key, openai_base_url?, anthropic_base_url? }` | `Provider` |
| GET | `/api/v1/providers` | — | `Provider[]` |
| GET | `/api/v1/providers/:id` | — | `Provider` |
| PATCH | `/api/v1/providers/:id` | `{ name?, api_key?, openai_base_url?, anthropic_base_url?, enabled? }` | `Provider` |
| DELETE | `/api/v1/providers/:id` | — | 204 No Content |

`Provider` = `{ id, name, api_key, openai_base_url?, anthropic_base_url?, enabled, created_at, updated_at }`

### Models (nested under providers)
| Method | Path | Request Body | Response |
|---|---|---|---|
| POST | `/api/v1/providers/:id/models` | `{ name, billing_type, input_price?, output_price?, request_price? }` | `Model` |
| PATCH | `/api/v1/providers/:id/models/:name` | `{ billing_type?, input_price?, output_price?, request_price?, enabled? }` | `Model` |
| DELETE | `/api/v1/providers/:id/models/:name` | — | 204 No Content |

`Model` = `{ name, provider_id, billing_type: "token"|"request", input_price, output_price, request_price, enabled, created_at }`

### Usage & Logs
| Method | Path | Query Params | Response |
|---|---|---|---|
| GET | `/api/v1/usage` | `key_id?, model_name?, since?, until?` | `UsageRecord[]` |
| GET | `/api/v1/logs` | `key_id?, model_name?, since?, until?, offset?, limit?` | `AuditLog[]` |

`UsageRecord` = `{ id, key_id, model_name, provider_id, protocol, input_tokens?, output_tokens?, cost, created_at }`
`AuditLog` = `{ id, key_id, model_name, provider_id, protocol, request_body, response_body, status_code, latency_ms, input_tokens?, output_tokens?, created_at }`

---

## File Structure

```
web/
├── package.json
├── tsconfig.json
├── tsconfig.node.json
├── vite.config.ts
├── index.html
├── src/
│   ├── main.tsx                    # Entry point, renders App
│   ├── App.tsx                     # Router setup, layout, auth guard
│   ├── api/
│   │   ├── client.ts               # Axios instance with auth interceptor
│   │   ├── keys.ts                 # API key CRUD functions
│   │   ├── providers.ts            # Provider CRUD functions
│   │   ├── models.ts               # Model CRUD functions (nested under provider)
│   │   ├── usage.ts                # Usage query function
│   │   └── logs.ts                 # Audit log query function
│   ├── types/
│   │   └── index.ts                # All TypeScript interfaces matching backend
│   ├── hooks/
│   │   ├── useKeys.ts              # React Query hooks for keys
│   │   ├── useProviders.ts         # React Query hooks for providers
│   │   ├── useModels.ts            # React Query hooks for models
│   │   ├── useUsage.ts             # React Query hook for usage
│   │   └── useLogs.ts              # React Query hook for logs
│   ├── pages/
│   │   ├── Dashboard.tsx           # Overview dashboard
│   │   ├── Keys.tsx                # API key list
│   │   ├── KeyDetail.tsx           # Key edit/detail
│   │   ├── Providers.tsx           # Provider list
│   │   ├── ProviderDetail.tsx      # Provider edit/detail with model management
│   │   ├── Usage.tsx               # Usage analytics
│   │   └── Logs.tsx                # Audit log browser
│   └── components/
│       ├── Layout.tsx              # App shell with sidebar navigation
│       ├── StatCard.tsx            # Dashboard stat display card
│       └── JsonViewer.tsx           # Collapsible JSON viewer for audit logs
```

---

## Task 1: Project scaffold with Vite + React + TypeScript

**Files:**
- Create: `web/package.json`
- Create: `web/tsconfig.json`
- Create: `web/tsconfig.node.json`
- Create: `web/vite.config.ts`
- Create: `web/index.html`
- Create: `web/src/main.tsx`
- Create: `web/src/App.tsx`
- Create: `web/src/vite-env.d.ts`

- [ ] **Step 1: Create package.json**

```json
{
  "name": "llm-gateway-web",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc -b && vite build",
    "preview": "vite preview"
  },
  "dependencies": {
    "react": "^18.3.1",
    "react-dom": "^18.3.1",
    "react-router-dom": "^6.28.0",
    "@tanstack/react-query": "^5.62.0",
    "antd": "^5.22.0",
    "@ant-design/icons": "^5.5.0",
    "recharts": "^2.15.0",
    "axios": "^1.7.0",
    "dayjs": "^1.11.0"
  },
  "devDependencies": {
    "@types/react": "^18.3.0",
    "@types/react-dom": "^18.3.0",
    "@vitejs/plugin-react": "^4.3.0",
    "typescript": "^5.7.0",
    "vite": "^6.0.0"
  }
}
```

- [ ] **Step 2: Create tsconfig.json**

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "isolatedModules": true,
    "moduleDetection": "force",
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true,
    "noUncheckedSideEffectImports": true
  },
  "include": ["src"]
}
```

- [ ] **Step 3: Create tsconfig.node.json**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["ES2023"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "isolatedModules": true,
    "moduleDetection": "force",
    "noEmit": true,
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true,
    "noUncheckedSideEffectImports": true
  },
  "include": ["vite.config.ts"]
}
```

- [ ] **Step 4: Create vite.config.ts**

```typescript
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  base: '/admin/',
  server: {
    port: 5173,
    proxy: {
      '/api': {
        target: 'http://localhost:8080',
        changeOrigin: true,
      },
    },
  },
  build: {
    outDir: 'dist',
  },
});
```

- [ ] **Step 5: Create index.html**

```html
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>LLM Gateway</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

- [ ] **Step 6: Create src/vite-env.d.ts**

```typescript
/// <reference types="vite/client" />
```

- [ ] **Step 7: Create src/main.tsx**

```tsx
import React from 'react';
import ReactDOM from 'react-dom/client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import App from './App';

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      refetchOnWindowFocus: false,
      retry: 1,
    },
  },
});

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <App />
    </QueryClientProvider>
  </React.StrictMode>,
);
```

- [ ] **Step 8: Create src/App.tsx (placeholder)**

```tsx
function App() {
  return <div>LLM Gateway Admin</div>;
}

export default App;
```

- [ ] **Step 9: Install dependencies and verify dev server starts**

Run: `cd web && npm install`
Run: `cd web && npm run dev`
Expected: Vite dev server starts on http://localhost:5173

- [ ] **Step 10: Commit**

```bash
git add web/
git commit -m "feat(web): scaffold React + TypeScript project with Vite"
```

---

## Task 2: TypeScript types and API client

**Files:**
- Create: `web/src/types/index.ts`
- Create: `web/src/api/client.ts`

- [ ] **Step 1: Create TypeScript types**

```typescript
// web/src/types/index.ts

export interface ApiKey {
  id: string;
  name: string;
  key_hash: string;
  rate_limit: number | null;
  budget_monthly: number | null;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateKeyRequest {
  name: string;
  rate_limit?: number | null;
  budget_monthly?: number | null;
}

export interface CreateKeyResponse {
  id: string;
  name: string;
  key: string;
  rate_limit: number | null;
  budget_monthly: number | null;
  enabled: boolean;
  created_at: string;
}

export interface UpdateKeyRequest {
  name?: string;
  rate_limit?: number | null;
  budget_monthly?: number | null;
  enabled?: boolean;
}

export interface Provider {
  id: string;
  name: string;
  api_key: string;
  openai_base_url: string | null;
  anthropic_base_url: string | null;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateProviderRequest {
  name: string;
  api_key: string;
  openai_base_url?: string | null;
  anthropic_base_url?: string | null;
}

export interface UpdateProviderRequest {
  name?: string;
  api_key?: string;
  openai_base_url?: string | null;
  anthropic_base_url?: string | null;
  enabled?: boolean;
}

export interface Model {
  name: string;
  provider_id: string;
  billing_type: 'token' | 'request';
  input_price: number;
  output_price: number;
  request_price: number;
  enabled: boolean;
  created_at: string;
}

export interface CreateModelRequest {
  name: string;
  billing_type: 'token' | 'request';
  input_price?: number;
  output_price?: number;
  request_price?: number;
}

export interface UpdateModelRequest {
  billing_type?: 'token' | 'request';
  input_price?: number;
  output_price?: number;
  request_price?: number;
  enabled?: boolean;
}

export interface UsageRecord {
  id: string;
  key_id: string;
  model_name: string;
  provider_id: string;
  protocol: 'openai' | 'anthropic';
  input_tokens: number | null;
  output_tokens: number | null;
  cost: number;
  created_at: string;
}

export interface UsageFilter {
  key_id?: string;
  model_name?: string;
  since?: string;
  until?: string;
}

export interface AuditLog {
  id: string;
  key_id: string;
  model_name: string;
  provider_id: string;
  protocol: 'openai' | 'anthropic';
  request_body: string;
  response_body: string;
  status_code: number;
  latency_ms: number;
  input_tokens: number | null;
  output_tokens: number | null;
  created_at: string;
}

export interface LogFilter {
  key_id?: string;
  model_name?: string;
  since?: string;
  until?: string;
  offset?: number;
  limit?: number;
}
```

- [ ] **Step 2: Create Axios client with auth interceptor**

```typescript
// web/src/api/client.ts
import axios from 'axios';

const TOKEN_KEY = 'llm_gateway_admin_token';

export const apiClient = axios.create({
  baseURL: '/api/v1',
  headers: {
    'Content-Type': 'application/json',
  },
});

// Request interceptor: attach admin token
apiClient.interceptors.request.use((config) => {
  const token = localStorage.getItem(TOKEN_KEY);
  if (token) {
    config.headers.Authorization = `Bearer ${token}`;
  }
  return config;
});

// Response interceptor: handle 401 (redirect to login)
apiClient.interceptors.response.use(
  (response) => response,
  (error) => {
    if (error.response?.status === 401) {
      localStorage.removeItem(TOKEN_KEY);
      window.location.href = '/admin/login';
    }
    return Promise.reject(error);
  },
);

export function setToken(token: string) {
  localStorage.setItem(TOKEN_KEY, token);
}

export function getToken(): string | null {
  return localStorage.getItem(TOKEN_KEY);
}

export function clearToken() {
  localStorage.removeItem(TOKEN_KEY);
}
```

- [ ] **Step 3: Verify TypeScript compiles**

Run: `cd web && npx tsc --noEmit`
Expected: No errors

- [ ] **Step 4: Commit**

```bash
git add web/src/types/ web/src/api/client.ts
git commit -m "feat(web): add TypeScript types and API client with auth"
```

---

## Task 3: API functions (keys, providers, models, usage, logs)

**Files:**
- Create: `web/src/api/keys.ts`
- Create: `web/src/api/providers.ts`
- Create: `web/src/api/models.ts`
- Create: `web/src/api/usage.ts`
- Create: `web/src/api/logs.ts`

- [ ] **Step 1: Create keys API**

```typescript
// web/src/api/keys.ts
import { apiClient } from './client';
import type { ApiKey, CreateKeyRequest, CreateKeyResponse, UpdateKeyRequest } from '../types';

export async function listKeys(): Promise<ApiKey[]> {
  const { data } = await apiClient.get<ApiKey[]>('/keys');
  return data;
}

export async function getKey(id: string): Promise<ApiKey> {
  const { data } = await apiClient.get<ApiKey>(`/keys/${id}`);
  return data;
}

export async function createKey(input: CreateKeyRequest): Promise<CreateKeyResponse> {
  const { data } = await apiClient.post<CreateKeyResponse>('/keys', input);
  return data;
}

export async function updateKey(id: string, input: UpdateKeyRequest): Promise<ApiKey> {
  const { data } = await apiClient.patch<ApiKey>(`/keys/${id}`, input);
  return data;
}

export async function deleteKey(id: string): Promise<void> {
  await apiClient.delete(`/keys/${id}`);
}
```

- [ ] **Step 2: Create providers API**

```typescript
// web/src/api/providers.ts
import { apiClient } from './client';
import type { Provider, CreateProviderRequest, UpdateProviderRequest } from '../types';

export async function listProviders(): Promise<Provider[]> {
  const { data } = await apiClient.get<Provider[]>('/providers');
  return data;
}

export async function getProvider(id: string): Promise<Provider> {
  const { data } = await apiClient.get<Provider>(`/providers/${id}`);
  return data;
}

export async function createProvider(input: CreateProviderRequest): Promise<Provider> {
  const { data } = await apiClient.post<Provider>('/providers', input);
  return data;
}

export async function updateProvider(id: string, input: UpdateProviderRequest): Promise<Provider> {
  const { data } = await apiClient.patch<Provider>(`/providers/${id}`, input);
  return data;
}

export async function deleteProvider(id: string): Promise<void> {
  await apiClient.delete(`/providers/${id}`);
}
```

- [ ] **Step 3: Create models API**

```typescript
// web/src/api/models.ts
import { apiClient } from './client';
import type { Model, CreateModelRequest, UpdateModelRequest } from '../types';

export async function createModel(providerId: string, input: CreateModelRequest): Promise<Model> {
  const { data } = await apiClient.post<Model>(`/providers/${providerId}/models`, input);
  return data;
}

export async function updateModel(providerId: string, modelName: string, input: UpdateModelRequest): Promise<Model> {
  const { data } = await apiClient.patch<Model>(`/providers/${providerId}/models/${modelName}`, input);
  return data;
}

export async function deleteModel(providerId: string, modelName: string): Promise<void> {
  await apiClient.delete(`/providers/${providerId}/models/${modelName}`);
}
```

- [ ] **Step 4: Create usage API**

```typescript
// web/src/api/usage.ts
import { apiClient } from './client';
import type { UsageRecord, UsageFilter } from '../types';

export async function queryUsage(filter: UsageFilter = {}): Promise<UsageRecord[]> {
  const params: Record<string, string> = {};
  if (filter.key_id) params.key_id = filter.key_id;
  if (filter.model_name) params.model_name = filter.model_name;
  if (filter.since) params.since = filter.since;
  if (filter.until) params.until = filter.until;
  const { data } = await apiClient.get<UsageRecord[]>('/usage', { params });
  return data;
}
```

- [ ] **Step 5: Create logs API**

```typescript
// web/src/api/logs.ts
import { apiClient } from './client';
import type { AuditLog, LogFilter } from '../types';

export async function queryLogs(filter: LogFilter = {}): Promise<AuditLog[]> {
  const params: Record<string, string | number> = {};
  if (filter.key_id) params.key_id = filter.key_id;
  if (filter.model_name) params.model_name = filter.model_name;
  if (filter.since) params.since = filter.since;
  if (filter.until) params.until = filter.until;
  if (filter.offset != null) params.offset = filter.offset;
  if (filter.limit != null) params.limit = filter.limit;
  const { data } = await apiClient.get<AuditLog[]>('/logs', { params });
  return data;
}
```

- [ ] **Step 6: Verify TypeScript compiles**

Run: `cd web && npx tsc --noEmit`
Expected: No errors

- [ ] **Step 7: Commit**

```bash
git add web/src/api/
git commit -m "feat(web): add API functions for keys, providers, models, usage, logs"
```

---

## Task 4: React Query hooks

**Files:**
- Create: `web/src/hooks/useKeys.ts`
- Create: `web/src/hooks/useProviders.ts`
- Create: `web/src/hooks/useModels.ts`
- Create: `web/src/hooks/useUsage.ts`
- Create: `web/src/hooks/useLogs.ts`

- [ ] **Step 1: Create useKeys hook**

```typescript
// web/src/hooks/useKeys.ts
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { listKeys, createKey, updateKey, deleteKey } from '../api/keys';
import type { CreateKeyRequest, UpdateKeyRequest } from '../types';
import { message } from 'antd';

export function useKeys() {
  return useQuery({
    queryKey: ['keys'],
    queryFn: listKeys,
  });
}

export function useCreateKey() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateKeyRequest) => createKey(input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['keys'] });
      message.success('API key created');
    },
    onError: () => {
      message.error('Failed to create API key');
    },
  });
}

export function useUpdateKey() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateKeyRequest }) => updateKey(id, input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['keys'] });
      message.success('API key updated');
    },
    onError: () => {
      message.error('Failed to update API key');
    },
  });
}

export function useDeleteKey() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => deleteKey(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['keys'] });
      message.success('API key deleted');
    },
    onError: () => {
      message.error('Failed to delete API key');
    },
  });
}
```

- [ ] **Step 2: Create useProviders hook**

```typescript
// web/src/hooks/useProviders.ts
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { listProviders, getProvider, createProvider, updateProvider, deleteProvider } from '../api/providers';
import type { CreateProviderRequest, UpdateProviderRequest } from '../types';
import { message } from 'antd';

export function useProviders() {
  return useQuery({
    queryKey: ['providers'],
    queryFn: listProviders,
  });
}

export function useProvider(id: string) {
  return useQuery({
    queryKey: ['providers', id],
    queryFn: () => getProvider(id),
    enabled: !!id,
  });
}

export function useCreateProvider() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateProviderRequest) => createProvider(input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['providers'] });
      message.success('Provider created');
    },
    onError: () => {
      message.error('Failed to create provider');
    },
  });
}

export function useUpdateProvider() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateProviderRequest }) => updateProvider(id, input),
    onSuccess: (_, variables) => {
      queryClient.invalidateQueries({ queryKey: ['providers'] });
      queryClient.invalidateQueries({ queryKey: ['providers', variables.id] });
      message.success('Provider updated');
    },
    onError: () => {
      message.error('Failed to update provider');
    },
  });
}

export function useDeleteProvider() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => deleteProvider(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['providers'] });
      message.success('Provider deleted');
    },
    onError: () => {
      message.error('Failed to delete provider');
    },
  });
}
```

- [ ] **Step 3: Create useModels hook**

```typescript
// web/src/hooks/useModels.ts
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { createModel, updateModel, deleteModel } from '../api/models';
import type { CreateModelRequest, UpdateModelRequest } from '../types';
import { message } from 'antd';

export function useCreateModel(providerId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateModelRequest) => createModel(providerId, input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['providers', providerId] });
      message.success('Model added');
    },
    onError: () => {
      message.error('Failed to add model');
    },
  });
}

export function useUpdateModel(providerId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ modelName, input }: { modelName: string; input: UpdateModelRequest }) =>
      updateModel(providerId, modelName, input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['providers', providerId] });
      message.success('Model updated');
    },
    onError: () => {
      message.error('Failed to update model');
    },
  });
}

export function useDeleteModel(providerId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (modelName: string) => deleteModel(providerId, modelName),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['providers', providerId] });
      message.success('Model deleted');
    },
    onError: () => {
      message.error('Failed to delete model');
    },
  });
}
```

- [ ] **Step 4: Create useUsage hook**

```typescript
// web/src/hooks/useUsage.ts
import { useQuery } from '@tanstack/react-query';
import { queryUsage } from '../api/usage';
import type { UsageFilter } from '../types';

export function useUsage(filter: UsageFilter) {
  return useQuery({
    queryKey: ['usage', filter],
    queryFn: () => queryUsage(filter),
  });
}
```

- [ ] **Step 5: Create useLogs hook**

```typescript
// web/src/hooks/useLogs.ts
import { useQuery } from '@tanstack/react-query';
import { queryLogs } from '../api/logs';
import type { LogFilter } from '../types';

export function useLogs(filter: LogFilter) {
  return useQuery({
    queryKey: ['logs', filter],
    queryFn: () => queryLogs(filter),
  });
}
```

- [ ] **Step 6: Verify TypeScript compiles**

Run: `cd web && npx tsc --noEmit`
Expected: No errors

- [ ] **Step 7: Commit**

```bash
git add web/src/hooks/
git commit -m "feat(web): add React Query hooks for all API endpoints"
```

---

## Task 5: Layout component with sidebar navigation

**Files:**
- Create: `web/src/components/Layout.tsx`
- Modify: `web/src/App.tsx`

- [ ] **Step 1: Create Layout component**

```tsx
// web/src/components/Layout.tsx
import { useState } from 'react';
import { Outlet, useNavigate, useLocation } from 'react-router-dom';
import { Layout, Menu, theme } from 'antd';
import {
  DashboardOutlined,
  KeyOutlined,
  CloudServerOutlined,
  BarChartOutlined,
  FileSearchOutlined,
  LogoutOutlined,
} from '@ant-design/icons';
import { clearToken } from '../api/client';

const { Header, Sider, Content } = Layout;

const menuItems = [
  { key: '/admin/', icon: <DashboardOutlined />, label: 'Dashboard' },
  { key: '/admin/keys', icon: <KeyOutlined />, label: 'API Keys' },
  { key: '/admin/providers', icon: <CloudServerOutlined />, label: 'Providers' },
  { key: '/admin/usage', icon: <BarChartOutlined />, label: 'Usage' },
  { key: '/admin/logs', icon: <FileSearchOutlined />, label: 'Logs' },
];

export default function AppLayout() {
  const [collapsed, setCollapsed] = useState(false);
  const navigate = useNavigate();
  const location = useLocation();
  const { token: { colorBgContainer, borderRadiusLG } } = theme.useToken();

  const handleLogout = () => {
    clearToken();
    navigate('/admin/login');
  };

  return (
    <Layout style={{ minHeight: '100vh' }}>
      <Sider collapsible collapsed={collapsed} onCollapse={setCollapsed}>
        <div style={{ height: 32, margin: 16, textAlign: 'center', color: '#fff', fontSize: collapsed ? 14 : 16, fontWeight: 'bold' }}>
          {collapsed ? 'GW' : 'LLM Gateway'}
        </div>
        <Menu
          theme="dark"
          mode="inline"
          selectedKeys={[location.pathname]}
          items={menuItems}
          onClick={({ key }) => navigate(key)}
        />
      </Sider>
      <Layout>
        <Header style={{ padding: '0 16px', background: colorBgContainer, display: 'flex', justifyContent: 'flex-end' }}>
          <a onClick={handleLogout} style={{ cursor: 'pointer' }}>
            <LogoutOutlined /> Logout
          </a>
        </Header>
        <Content style={{ margin: 16 }}>
          <div style={{ padding: 24, minHeight: 360, background: colorBgContainer, borderRadius: borderRadiusLG }}>
            <Outlet />
          </div>
        </Content>
      </Layout>
    </Layout>
  );
}
```

- [ ] **Step 2: Create Login page**

```tsx
// web/src/pages/Login.tsx
import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Form, Input, Button, Card, Typography, message } from 'antd';
import { setToken } from '../api/client';
import { apiClient } from '../api/client';

const { Title } = Typography;

export default function Login() {
  const [loading, setLoading] = useState(false);
  const navigate = useNavigate();

  const onFinish = async (values: { token: string }) => {
    setLoading(true);
    setToken(values.token);
    try {
      // Verify token by making a test request
      await apiClient.get('/keys');
      navigate('/admin/');
    } catch {
      message.error('Invalid admin token');
      setToken('');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', minHeight: '100vh' }}>
      <Card style={{ width: 400 }}>
        <Title level={3} style={{ textAlign: 'center' }}>LLM Gateway</Title>
        <Form onFinish={onFinish} layout="vertical">
          <Form.Item name="token" label="Admin Token" rules={[{ required: true }]}>
            <Input.Password placeholder="Enter admin token" />
          </Form.Item>
          <Form.Item>
            <Button type="primary" htmlType="submit" loading={loading} block>
              Login
            </Button>
          </Form.Item>
        </Form>
      </Card>
    </div>
  );
}
```

- [ ] **Step 3: Update App.tsx with routing**

```tsx
// web/src/App.tsx
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { getToken } from './api/client';
import Layout from './components/Layout';
import Login from './pages/Login';
import Dashboard from './pages/Dashboard';
import Keys from './pages/Keys';
import KeyDetail from './pages/KeyDetail';
import Providers from './pages/Providers';
import ProviderDetail from './pages/ProviderDetail';
import Usage from './pages/Usage';
import Logs from './pages/Logs';

function PrivateRoute({ children }: { children: React.ReactNode }) {
  const token = getToken();
  if (!token) return <Navigate to="/admin/login" replace />;
  return <>{children}</>;
}

function App() {
  return (
    <BrowserRouter basename="/admin">
      <Routes>
        <Route path="/login" element={<Login />} />
        <Route path="/" element={<PrivateRoute><Layout /></PrivateRoute>}>
          <Route index element={<Dashboard />} />
          <Route path="keys" element={<Keys />} />
          <Route path="keys/:id" element={<KeyDetail />} />
          <Route path="providers" element={<Providers />} />
          <Route path="providers/:id" element={<ProviderDetail />} />
          <Route path="usage" element={<Usage />} />
          <Route path="logs" element={<Logs />} />
        </Route>
        <Route path="*" element={<Navigate to="/admin/" replace />} />
      </Routes>
    </BrowserRouter>
  );
}

export default App;
```

- [ ] **Step 4: Create placeholder page components**

Create minimal placeholder files for each page:

```tsx
// web/src/pages/Dashboard.tsx
export default function Dashboard() { return <div>Dashboard</div>; }

// web/src/pages/Keys.tsx
export default function Keys() { return <div>Keys</div>; }

// web/src/pages/KeyDetail.tsx
export default function KeyDetail() { return <div>Key Detail</div>; }

// web/src/pages/Providers.tsx
export default function Providers() { return <div>Providers</div>; }

// web/src/pages/ProviderDetail.tsx
export default function ProviderDetail() { return <div>Provider Detail</div>; }

// web/src/pages/Usage.tsx
export default function Usage() { return <div>Usage</div>; }

// web/src/pages/Logs.tsx
export default function Logs() { return <div>Logs</div>; }
```

- [ ] **Step 5: Verify TypeScript compiles**

Run: `cd web && npx tsc --noEmit`
Expected: No errors

- [ ] **Step 6: Commit**

```bash
git add web/src/
git commit -m "feat(web): add layout, login, routing, and page placeholders"
```

---

## Task 6: API Keys page (list + create)

**Files:**
- Modify: `web/src/pages/Keys.tsx`

- [ ] **Step 1: Implement Keys page**

```tsx
// web/src/pages/Keys.tsx
import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Table, Button, Modal, Form, Input, InputNumber, Space, Tag, Typography, message } from 'antd';
import { PlusOutlined } from '@ant-design/icons';
import { useKeys, useCreateKey } from '../hooks/useKeys';
import type { CreateKeyResponse } from '../types';

const { Text, Paragraph } = Typography;

export default function Keys() {
  const { data: keys, isLoading } = useKeys();
  const createKeyMutation = useCreateKey();
  const navigate = useNavigate();
  const [createOpen, setCreateOpen] = useState(false);
  const [createdKey, setCreatedKey] = useState<string | null>(null);
  const [form] = Form.useForm();

  const handleCreate = async (values: { name: string; rate_limit?: number; budget_monthly?: number }) => {
    const result: CreateKeyResponse = await createKeyMutation.mutateAsync({
      name: values.name,
      rate_limit: values.rate_limit ?? null,
      budget_monthly: values.budget_monthly ?? null,
    });
    setCreatedKey(result.key);
    form.resetFields();
  };

  const copyKey = () => {
    if (createdKey) {
      navigator.clipboard.writeText(createdKey);
      message.success('Key copied to clipboard');
    }
  };

  const columns = [
    {
      title: 'Name',
      dataIndex: 'name',
      key: 'name',
      render: (name: string, record: { id: string }) => (
        <a onClick={() => navigate(`/admin/keys/${record.id}`)}>{name}</a>
      ),
    },
    {
      title: 'Status',
      dataIndex: 'enabled',
      key: 'enabled',
      render: (enabled: boolean) => (
        <Tag color={enabled ? 'green' : 'red'}>{enabled ? 'Active' : 'Disabled'}</Tag>
      ),
    },
    {
      title: 'Rate Limit (RPM)',
      dataIndex: 'rate_limit',
      key: 'rate_limit',
      render: (v: number | null) => v ?? 'Unlimited',
    },
    {
      title: 'Monthly Budget',
      dataIndex: 'budget_monthly',
      key: 'budget_monthly',
      render: (v: number | null) => v != null ? `$${v.toFixed(2)}` : 'Unlimited',
    },
    {
      title: 'Created',
      dataIndex: 'created_at',
      key: 'created_at',
      render: (v: string) => new Date(v).toLocaleDateString(),
    },
  ];

  return (
    <>
      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 16 }}>
        <Typography.Title level={4} style={{ margin: 0 }}>API Keys</Typography.Title>
        <Button type="primary" icon={<PlusOutlined />} onClick={() => setCreateOpen(true)}>
          Create Key
        </Button>
      </div>

      <Table dataSource={keys} columns={columns} rowKey="id" loading={isLoading} />

      <Modal
        title="Create API Key"
        open={createOpen}
        onCancel={() => { setCreateOpen(false); setCreatedKey(null); }}
        footer={createdKey ? [
          <Button key="copy" type="primary" onClick={copyKey}>Copy Key</Button>,
          <Button key="done" onClick={() => { setCreateOpen(false); setCreatedKey(null); }}>Done</Button>,
        ] : undefined}
      >
        {createdKey ? (
          <div>
            <Text type="secondary">Save this key now. It won't be shown again.</Text>
            <Paragraph copyable style={{ marginTop: 8, fontFamily: 'monospace', fontSize: 14 }}>
              {createdKey}
            </Paragraph>
          </div>
        ) : (
          <Form form={form} layout="vertical" onFinish={handleCreate}>
            <Form.Item name="name" label="Name" rules={[{ required: true, message: 'Enter a name' }]}>
              <Input placeholder="e.g., production-app" />
            </Form.Item>
            <Space>
              <Form.Item name="rate_limit" label="Rate Limit (RPM)">
                <InputNumber min={1} placeholder="Unlimited" style={{ width: 150 }} />
              </Form.Item>
              <Form.Item name="budget_monthly" label="Monthly Budget ($)">
                <InputNumber min={0} step={0.01} placeholder="Unlimited" style={{ width: 150 }} />
              </Form.Item>
            </Space>
            <Form.Item>
              <Button type="primary" htmlType="submit" loading={createKeyMutation.isPending}>
                Create
              </Button>
            </Form.Item>
          </Form>
        )}
      </Modal>
    </>
  );
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd web && npx tsc --noEmit`

- [ ] **Step 3: Commit**

```bash
git add web/src/pages/Keys.tsx
git commit -m "feat(web): implement API Keys list page with create modal"
```

---

## Task 7: Key Detail page (edit + delete)

**Files:**
- Modify: `web/src/pages/KeyDetail.tsx`

- [ ] **Step 1: Implement KeyDetail page**

```tsx
// web/src/pages/KeyDetail.tsx
import { useParams, useNavigate } from 'react-router-dom';
import { Card, Form, Input, InputNumber, Switch, Button, Space, Typography, Popconfirm, message } from 'antd';
import { ArrowLeftOutlined } from '@ant-design/icons';
import { useProvider } from '../hooks/useProviders';
import { useKey, useUpdateKey, useDeleteKey } from '../hooks/useKeys';

const { Title } = Typography;

export default function KeyDetail() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { data: key, isLoading } = useKey(id!);
  const updateMutation = useUpdateKey();
  const deleteMutation = useDeleteKey();
  const [form] = Form.useForm();

  if (isLoading) return <div>Loading...</div>;
  if (!key) return <div>Key not found</div>;

  const handleUpdate = async (values: any) => {
    await updateMutation.mutateAsync({
      id: key.id,
      input: {
        name: values.name,
        rate_limit: values.rate_limit ?? null,
        budget_monthly: values.budget_monthly ?? null,
        enabled: values.enabled,
      },
    });
  };

  const handleDelete = async () => {
    await deleteMutation.mutateAsync(key.id);
    navigate('/admin/keys');
  };

  return (
    <div>
      <Button icon={<ArrowLeftOutlined />} onClick={() => navigate('/admin/keys')} style={{ marginBottom: 16 }}>
        Back to Keys
      </Button>

      <Card title={<Title level={4} style={{ margin: 0 }}>Edit Key: {key.name}</Title>}>
        <Form
          form={form}
          layout="vertical"
          initialValues={{
            name: key.name,
            rate_limit: key.rate_limit,
            budget_monthly: key.budget_monthly,
            enabled: key.enabled,
          }}
          onFinish={handleUpdate}
          style={{ maxWidth: 500 }}
        >
          <Form.Item name="name" label="Name" rules={[{ required: true }]}>
            <Input />
          </Form.Item>
          <Form.Item name="enabled" label="Enabled" valuePropName="checked">
            <Switch />
          </Form.Item>
          <Form.Item name="rate_limit" label="Rate Limit (RPM, null = unlimited)">
            <InputNumber min={1} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="budget_monthly" label="Monthly Budget ($, null = unlimited)">
            <InputNumber min={0} step={0.01} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item>
            <Space>
              <Button type="primary" htmlType="submit" loading={updateMutation.isPending}>Save</Button>
              <Popconfirm title="Delete this key?" onConfirm={handleDelete}>
                <Button danger>Delete Key</Button>
              </Popconfirm>
            </Space>
          </Form.Item>
        </Form>
      </Card>
    </div>
  );
}
```

- [ ] **Step 2: Verify and commit**

Run: `cd web && npx tsc --noEmit`

```bash
git add web/src/pages/KeyDetail.tsx
git commit -m "feat(web): implement Key detail/edit page"
```

---

## Task 8: Providers page (list + create)

**Files:**
- Modify: `web/src/pages/Providers.tsx`

- [ ] **Step 1: Implement Providers page**

```tsx
// web/src/pages/Providers.tsx
import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Table, Button, Modal, Form, Input, Space, Tag, Typography } from 'antd';
import { PlusOutlined } from '@ant-design/icons';
import { useProviders, useCreateProvider } from '../hooks/useProviders';

const { Title } = Typography;

export default function Providers() {
  const { data: providers, isLoading } = useProviders();
  const createMutation = useCreateProvider();
  const navigate = useNavigate();
  const [createOpen, setCreateOpen] = useState(false);
  const [form] = Form.useForm();

  const handleCreate = async (values: { name: string; api_key: string; openai_base_url?: string; anthropic_base_url?: string }) => {
    await createMutation.mutateAsync({
      name: values.name,
      api_key: values.api_key,
      openai_base_url: values.openai_base_url || null,
      anthropic_base_url: values.anthropic_base_url || null,
    });
    form.resetFields();
    setCreateOpen(false);
  };

  const columns = [
    {
      title: 'Name',
      dataIndex: 'name',
      key: 'name',
      render: (name: string, record: { id: string }) => (
        <a onClick={() => navigate(`/admin/providers/${record.id}`)}>{name}</a>
      ),
    },
    {
      title: 'Protocols',
      key: 'protocols',
      render: (_: unknown, record: { openai_base_url: string | null; anthropic_base_url: string | null }) => (
        <Space>
          {record.openai_base_url && <Tag color="blue">OpenAI</Tag>}
          {record.anthropic_base_url && <Tag color="purple">Anthropic</Tag>}
        </Space>
      ),
    },
    {
      title: 'Status',
      dataIndex: 'enabled',
      key: 'enabled',
      render: (enabled: boolean) => (
        <Tag color={enabled ? 'green' : 'red'}>{enabled ? 'Active' : 'Disabled'}</Tag>
      ),
    },
    {
      title: 'Created',
      dataIndex: 'created_at',
      key: 'created_at',
      render: (v: string) => new Date(v).toLocaleDateString(),
    },
  ];

  return (
    <>
      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 16 }}>
        <Title level={4} style={{ margin: 0 }}>Providers</Title>
        <Button type="primary" icon={<PlusOutlined />} onClick={() => setCreateOpen(true)}>
          Add Provider
        </Button>
      </div>

      <Table dataSource={providers} columns={columns} rowKey="id" loading={isLoading} />

      <Modal
        title="Add Provider"
        open={createOpen}
        onCancel={() => setCreateOpen(false)}
        footer={null}
      >
        <Form form={form} layout="vertical" onFinish={handleCreate}>
          <Form.Item name="name" label="Name" rules={[{ required: true }]}>
            <Input placeholder="e.g., OpenAI" />
          </Form.Item>
          <Form.Item name="api_key" label="API Key" rules={[{ required: true }]}>
            <Input.Password placeholder="Upstream provider API key" />
          </Form.Item>
          <Form.Item name="openai_base_url" label="OpenAI Base URL">
            <Input placeholder="https://api.openai.com/v1" />
          </Form.Item>
          <Form.Item name="anthropic_base_url" label="Anthropic Base URL">
            <Input placeholder="https://api.anthropic.com" />
          </Form.Item>
          <Form.Item>
            <Button type="primary" htmlType="submit" loading={createMutation.isPending}>
              Create
            </Button>
          </Form.Item>
        </Form>
      </Modal>
    </>
  );
}
```

- [ ] **Step 2: Verify and commit**

Run: `cd web && npx tsc --noEmit`

```bash
git add web/src/pages/Providers.tsx
git commit -m "feat(web): implement Providers list page with create modal"
```

---

## Task 9: Provider Detail page (edit + model management)

**Files:**
- Modify: `web/src/pages/ProviderDetail.tsx`

- [ ] **Step 1: Implement ProviderDetail page**

```tsx
// web/src/pages/ProviderDetail.tsx
import { useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import {
  Card, Form, Input, Switch, Button, Space, Table, Modal,
  Popconfirm, Typography, Select, InputNumber, Tag, Divider, message,
} from 'antd';
import { ArrowLeftOutlined, PlusOutlined } from '@ant-design/icons';
import { useProvider, useUpdateProvider, useDeleteProvider } from '../hooks/useProviders';
import { useCreateModel, useUpdateModel, useDeleteModel } from '../hooks/useModels';
import type { Model, CreateModelRequest, UpdateModelRequest } from '../types';

const { Title } = Typography;

export default function ProviderDetail() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { data: provider, isLoading } = useProvider(id!);
  const updateMutation = useUpdateProvider();
  const deleteMutation = useDeleteProvider();
  const createModelMutation = useCreateModel(id!);
  const updateModelMutation = useUpdateModel(id!);
  const deleteModelMutation = useDeleteModel(id!);

  const [form] = Form.useForm();
  const [modelModalOpen, setModelModalOpen] = useState(false);
  const [editingModel, setEditingModel] = useState<Model | null>(null);
  const [modelForm] = Form.useForm();

  if (isLoading) return <div>Loading...</div>;
  if (!provider) return <div>Provider not found</div>;

  const handleUpdateProvider = async (values: any) => {
    await updateMutation.mutateAsync({
      id: provider.id,
      input: {
        name: values.name,
        api_key: values.api_key,
        openai_base_url: values.openai_base_url || null,
        anthropic_base_url: values.anthropic_base_url || null,
        enabled: values.enabled,
      },
    });
  };

  const handleDeleteProvider = async () => {
    await deleteMutation.mutateAsync(provider.id);
    navigate('/admin/providers');
  };

  const openAddModel = () => {
    setEditingModel(null);
    modelForm.resetFields();
    setModelModalOpen(true);
  };

  const openEditModel = (model: Model) => {
    setEditingModel(model);
    modelForm.setFieldsValue(model);
    setModelModalOpen(true);
  };

  const handleSaveModel = async (values: any) => {
    const input: CreateModelRequest = {
      name: values.name,
      billing_type: values.billing_type,
      input_price: values.input_price ?? 0,
      output_price: values.output_price ?? 0,
      request_price: values.request_price ?? 0,
    };

    if (editingModel) {
      const updateInput: UpdateModelRequest = {
        billing_type: values.billing_type,
        input_price: values.input_price,
        output_price: values.output_price,
        request_price: values.request_price,
        enabled: values.enabled,
      };
      await updateModelMutation.mutateAsync({ modelName: editingModel.name, input: updateInput });
    } else {
      await createModelMutation.mutateAsync(input);
    }
    setModelModalOpen(false);
  };

  const handleDeleteModel = async (modelName: string) => {
    await deleteModelMutation.mutateAsync(modelName);
  };

  const modelColumns = [
    { title: 'Name', dataIndex: 'name', key: 'name' },
    {
      title: 'Billing', dataIndex: 'billing_type', key: 'billing_type',
      render: (v: string) => <Tag color={v === 'token' ? 'blue' : 'green'}>{v}</Tag>,
    },
    {
      title: 'Input Price ($/1M)', dataIndex: 'input_price', key: 'input_price',
      render: (v: number) => v.toFixed(2),
    },
    {
      title: 'Output Price ($/1M)', dataIndex: 'output_price', key: 'output_price',
      render: (v: number) => v.toFixed(2),
    },
    {
      title: 'Status', dataIndex: 'enabled', key: 'enabled',
      render: (enabled: boolean) => <Tag color={enabled ? 'green' : 'red'}>{enabled ? 'Active' : 'Disabled'}</Tag>,
    },
    {
      title: 'Actions', key: 'actions',
      render: (_: unknown, record: Model) => (
        <Space>
          <a onClick={() => openEditModel(record)}>Edit</a>
          <Popconfirm title={`Delete model "${record.name}"?`} onConfirm={() => handleDeleteModel(record.name)}>
            <a style={{ color: '#ff4d4f' }}>Delete</a>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  return (
    <div>
      <Button icon={<ArrowLeftOutlined />} onClick={() => navigate('/admin/providers')} style={{ marginBottom: 16 }}>
        Back to Providers
      </Button>

      <Card title={<Title level={4} style={{ margin: 0 }}>Provider: {provider.name}</Title>} style={{ marginBottom: 16 }}>
        <Form
          form={form}
          layout="vertical"
          initialValues={{
            name: provider.name,
            api_key: provider.api_key,
            openai_base_url: provider.openai_base_url,
            anthropic_base_url: provider.anthropic_base_url,
            enabled: provider.enabled,
          }}
          onFinish={handleUpdateProvider}
          style={{ maxWidth: 500 }}
        >
          <Form.Item name="name" label="Name" rules={[{ required: true }]}>
            <Input />
          </Form.Item>
          <Form.Item name="api_key" label="API Key" rules={[{ required: true }]}>
            <Input.Password />
          </Form.Item>
          <Form.Item name="openai_base_url" label="OpenAI Base URL">
            <Input />
          </Form.Item>
          <Form.Item name="anthropic_base_url" label="Anthropic Base URL">
            <Input />
          </Form.Item>
          <Form.Item name="enabled" label="Enabled" valuePropName="checked">
            <Switch />
          </Form.Item>
          <Form.Item>
            <Space>
              <Button type="primary" htmlType="submit" loading={updateMutation.isPending}>Save</Button>
              <Popconfirm title="Delete this provider and all its models?" onConfirm={handleDeleteProvider}>
                <Button danger>Delete Provider</Button>
              </Popconfirm>
            </Space>
          </Form.Item>
        </Form>
      </Card>

      <Card
        title={<Title level={4} style={{ margin: 0 }}>Models</Title>}
        extra={<Button type="primary" icon={<PlusOutlined />} onClick={openAddModel}>Add Model</Button>}
      >
        <Table dataSource={[]} columns={modelColumns} rowKey="name" pagination={false} />
      </Card>

      <Modal
        title={editingModel ? `Edit Model: ${editingModel.name}` : 'Add Model'}
        open={modelModalOpen}
        onCancel={() => setModelModalOpen(false)}
        footer={null}
      >
        <Form form={modelForm} layout="vertical" onFinish={handleSaveModel}>
          <Form.Item name="name" label="Model Name" rules={[{ required: true }]} hidden={!!editingModel}>
            <Input placeholder="e.g., gpt-4o" />
          </Form.Item>
          <Form.Item name="billing_type" label="Billing Type" rules={[{ required: true }]}>
            <Select options={[
              { value: 'token', label: 'Token-based' },
              { value: 'request', label: 'Request-based' },
            ]} />
          </Form.Item>
          <Space>
            <Form.Item name="input_price" label="Input Price ($/1M tokens)">
              <InputNumber min={0} step={0.01} style={{ width: 150 }} />
            </Form.Item>
            <Form.Item name="output_price" label="Output Price ($/1M tokens)">
              <InputNumber min={0} step={0.01} style={{ width: 150 }} />
            </Form.Item>
            <Form.Item name="request_price" label="Request Price ($/req)">
              <InputNumber min={0} step={0.01} style={{ width: 150 }} />
            </Form.Item>
          </Space>
          {editingModel && (
            <Form.Item name="enabled" label="Enabled" valuePropName="checked">
              <Switch />
            </Form.Item>
          )}
          <Form.Item>
            <Button type="primary" htmlType="submit">
              {editingModel ? 'Update' : 'Create'}
            </Button>
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
}
```

Note: The models table `dataSource` is empty `[]` because the backend's provider detail endpoint doesn't return nested models. Models are managed through separate API calls. To populate the table, you'd need a `GET /api/v1/providers/:id/models` endpoint (not yet implemented in the backend). For now, the model CRUD modals still work — they call the correct API endpoints. When the backend adds the list-models-by-provider endpoint, update `dataSource` accordingly.

- [ ] **Step 2: Verify and commit**

Run: `cd web && npx tsc --noEmit`

```bash
git add web/src/pages/ProviderDetail.tsx
git commit -m "feat(web): implement Provider detail page with model management"
```

---

## Task 10: Dashboard page

**Files:**
- Create: `web/src/components/StatCard.tsx`
- Modify: `web/src/pages/Dashboard.tsx`

- [ ] **Step 1: Create StatCard component**

```tsx
// web/src/components/StatCard.tsx
import { Card, Statistic } from 'antd';

interface StatCardProps {
  title: string;
  value: number | string;
  prefix?: React.ReactNode;
  suffix?: string;
}

export default function StatCard({ title, value, prefix, suffix }: StatCardProps) {
  return (
    <Card>
      <Statistic title={title} value={value} prefix={prefix} suffix={suffix} />
    </Card>
  );
}
```

- [ ] **Step 2: Implement Dashboard page**

```tsx
// web/src/pages/Dashboard.tsx
import { Typography, Row, Col, Table, Tag } from 'antd';
import { DollarOutlined, MessageOutlined, ApiOutlined } from '@ant-design/icons';
import StatCard from '../components/StatCard';
import { useLogs } from '../hooks/useLogs';
import { useUsage } from '../hooks/useUsage';
import dayjs from 'dayjs';

const { Title } = Typography;

export default function Dashboard() {
  const today = dayjs().startOf('day').toISOString();
  const monthStart = dayjs().startOf('month').toISOString();

  const { data: allUsage } = useUsage({});
  const { data: todayUsage } = useUsage({ since: today });
  const { data: monthUsage } = useUsage({ since: monthStart });
  const { data: recentLogs } = useLogs({ limit: 20 });

  const todayRequests = todayUsage?.length ?? 0;
  const todayCost = todayUsage?.reduce((sum, r) => sum + r.cost, 0) ?? 0;
  const monthCost = monthUsage?.reduce((sum, r) => sum + r.cost, 0) ?? 0;
  const totalModels = new Set(allUsage?.map(r => r.model_name)).size;

  const logColumns = [
    { title: 'Time', dataIndex: 'created_at', key: 'created_at', width: 180,
      render: (v: string) => new Date(v).toLocaleString(),
    },
    { title: 'Model', dataIndex: 'model_name', key: 'model_name' },
    { title: 'Protocol', dataIndex: 'protocol', key: 'protocol',
      render: (v: string) => <Tag color={v === 'openai' ? 'blue' : 'purple'}>{v}</Tag>,
    },
    { title: 'Status', dataIndex: 'status_code', key: 'status_code',
      render: (v: number) => <Tag color={v < 400 ? 'green' : 'red'}>{v}</Tag>,
    },
    { title: 'Tokens', key: 'tokens',
      render: (_: unknown, r: { input_tokens: number | null; output_tokens: number | null }) =>
        `${r.input_tokens ?? 0} + ${r.output_tokens ?? 0}`,
    },
    { title: 'Cost', dataIndex: 'cost', key: 'cost',
      render: (v: number) => `$${v.toFixed(6)}`,
    },
    { title: 'Latency', dataIndex: 'latency_ms', key: 'latency_ms',
      render: (v: number) => `${v}ms`,
    },
  ];

  return (
    <div>
      <Title level={4}>Dashboard</Title>
      <Row gutter={16} style={{ marginBottom: 24 }}>
        <Col span={6}>
          <StatCard title="Today's Requests" value={todayRequests} prefix={<MessageOutlined />} />
        </Col>
        <Col span={6}>
          <StatCard title="Today's Cost" value={`$${todayCost.toFixed(4)}`} prefix={<DollarOutlined />} />
        </Col>
        <Col span={6}>
          <StatCard title="Monthly Cost" value={`$${monthCost.toFixed(2)}`} prefix={<DollarOutlined />} />
        </Col>
        <Col span={6}>
          <StatCard title="Active Models" value={totalModels} prefix={<ApiOutlined />} />
        </Col>
      </Row>

      <Title level={5}>Recent Requests</Title>
      <Table
        dataSource={recentLogs}
        columns={logColumns}
        rowKey="id"
        size="small"
        pagination={false}
        scroll={{ x: 700 }}
      />
    </div>
  );
}
```

- [ ] **Step 3: Verify and commit**

Run: `cd web && npx tsc --noEmit`

```bash
git add web/src/components/StatCard.tsx web/src/pages/Dashboard.tsx
git commit -m "feat(web): implement Dashboard page with stats and recent requests"
```

---

## Task 11: Usage page

**Files:**
- Modify: `web/src/pages/Usage.tsx`

- [ ] **Step 1: Implement Usage page**

```tsx
// web/src/pages/Usage.tsx
import { useState } from 'react';
import { Typography, DatePicker, Select, Table, Card, Row, Col, Statistic } from 'antd';
import { DollarOutlined, MessageOutlined } from '@ant-design/icons';
import { BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from 'recharts';
import { useUsage } from '../hooks/useUsage';
import { useKeys } from '../hooks/useKeys';
import dayjs, { Dayjs } from 'dayjs';
import type { UsageRecord } from '../types';

const { RangePicker } = DatePicker;
const { Title } = Typography;

export default function Usage() {
  const [dateRange, setDateRange] = useState<[Dayjs | null, Dayjs | null] | null>(null);
  const [keyFilter, setKeyFilter] = useState<string | undefined>(undefined);

  const since = dateRange?.[0]?.toISOString();
  const until = dateRange?.[1]?.toISOString();

  const { data: usage, isLoading } = useUsage({ since, until, key_id: keyFilter });
  const { data: keys } = useKeys();

  const totalCost = usage?.reduce((sum, r) => sum + r.cost, 0) ?? 0;
  const totalRequests = usage?.length ?? 0;
  const totalInputTokens = usage?.reduce((sum, r) => sum + (r.input_tokens ?? 0), 0) ?? 0;
  const totalOutputTokens = usage?.reduce((sum, r) => sum + (r.output_tokens ?? 0), 0) ?? 0;

  // Aggregate by model for chart
  const byModel: Record<string, { model: string; requests: number; cost: number }> = {};
  usage?.forEach((r) => {
    if (!byModel[r.model_name]) {
      byModel[r.model_name] = { model: r.model_name, requests: 0, cost: 0 };
    }
    byModel[r.model_name].requests += 1;
    byModel[r.model_name].cost += r.cost;
  });
  const chartData = Object.values(byModel).sort((a, b) => b.cost - a.cost);

  const columns = [
    { title: 'Time', dataIndex: 'created_at', key: 'created_at',
      render: (v: string) => new Date(v).toLocaleString(),
    },
    { title: 'Key ID', dataIndex: 'key_id', key: 'key_id',
      render: (v: string) => v.substring(0, 8) + '...',
    },
    { title: 'Model', dataIndex: 'model_name', key: 'model_name' },
    { title: 'Protocol', dataIndex: 'protocol', key: 'protocol' },
    { title: 'Input Tokens', dataIndex: 'input_tokens', key: 'input_tokens', render: (v: number | null) => v ?? '-' },
    { title: 'Output Tokens', dataIndex: 'output_tokens', key: 'output_tokens', render: (v: number | null) => v ?? '-' },
    { title: 'Cost', dataIndex: 'cost', key: 'cost', render: (v: number) => `$${v.toFixed(6)}` },
  ];

  return (
    <div>
      <Title level={4}>Usage</Title>

      <Card style={{ marginBottom: 16 }}>
        <Row gutter={16} align="middle">
          <Col>
            <RangePicker
              onChange={(dates) => setDateRange(dates as [Dayjs | null, Dayjs | null] | null)}
            />
          </Col>
          <Col>
            <Select
              placeholder="Filter by API Key"
              allowClear
              style={{ width: 200 }}
              onChange={(v) => setKeyFilter(v)}
              options={keys?.map(k => ({ value: k.id, label: k.name })) ?? []}
            />
          </Col>
        </Row>
      </Card>

      <Row gutter={16} style={{ marginBottom: 16 }}>
        <Col span={6}><Card><Statistic title="Total Cost" value={`$${totalCost.toFixed(4)}`} prefix={<DollarOutlined />} /></Card></Col>
        <Col span={6}><Card><Statistic title="Total Requests" value={totalRequests} prefix={<MessageOutlined />} /></Card></Col>
        <Col span={6}><Card><Statistic title="Input Tokens" value={totalInputTokens} /></Card></Col>
        <Col span={6}><Card><Statistic title="Output Tokens" value={totalOutputTokens} /></Card></Col>
      </Row>

      {chartData.length > 0 && (
        <Card title="Cost by Model" style={{ marginBottom: 16 }}>
          <ResponsiveContainer width="100%" height={300}>
            <BarChart data={chartData}>
              <CartesianGrid strokeDasharray="3 3" />
              <XAxis dataKey="model" />
              <YAxis />
              <Tooltip formatter={(value: number) => `$${value.toFixed(4)}`} />
              <Bar dataKey="cost" fill="#1890ff" name="Cost ($)" />
            </BarChart>
          </ResponsiveContainer>
        </Card>
      )}

      <Table dataSource={usage} columns={columns} rowKey="id" loading={isLoading} size="small" scroll={{ x: 800 }} />
    </div>
  );
}
```

- [ ] **Step 2: Verify and commit**

Run: `cd web && npx tsc --noEmit`

```bash
git add web/src/pages/Usage.tsx
git commit -m "feat(web): implement Usage page with filters, stats, and chart"
```

---

## Task 12: Logs page

**Files:**
- Create: `web/src/components/JsonViewer.tsx`
- Modify: `web/src/pages/Logs.tsx`

- [ ] **Step 1: Create JsonViewer component**

```tsx
// web/src/components/JsonViewer.tsx
import { useState } from 'react';
import { Typography } from 'antd';

const { Paragraph } = Typography;

interface JsonViewerProps {
  data: string;
  maxHeight?: number;
}

export default function JsonViewer({ data, maxHeight = 400 }: JsonViewerProps) {
  const [collapsed, setCollapsed] = useState(true);

  let formatted: string;
  try {
    formatted = JSON.stringify(JSON.parse(data), null, 2);
  } catch {
    formatted = data;
  }

  return (
    <div>
      <a onClick={() => setCollapsed(!collapsed)} style={{ marginBottom: 8 }}>
        {collapsed ? 'Expand' : 'Collapse'}
      </a>
      <Paragraph
        style={{
          maxHeight: collapsed ? 100 : maxHeight,
          overflow: 'auto',
          background: '#f5f5f5',
          padding: 8,
          borderRadius: 4,
          fontFamily: 'monospace',
          fontSize: 12,
          whiteSpace: 'pre-wrap',
          wordBreak: 'break-all',
        }}
      >
        {formatted}
      </Paragraph>
    </div>
  );
}
```

- [ ] **Step 2: Implement Logs page**

```tsx
// web/src/pages/Logs.tsx
import { useState } from 'react';
import { Typography, DatePicker, Select, Table, Card, Row, Col, Tag, Modal, Drawer } from 'antd';
import { useLogs } from '../hooks/useLogs';
import { useKeys } from '../hooks/useKeys';
import JsonViewer from '../components/JsonViewer';
import dayjs, { Dayjs } from 'dayjs';
import type { AuditLog } from '../types';

const { RangePicker } = DatePicker;
const { Title } = Typography;

export default function Logs() {
  const [dateRange, setDateRange] = useState<[Dayjs | null, Dayjs | null] | null>(null);
  const [keyFilter, setKeyFilter] = useState<string | undefined>(undefined);
  const [selectedLog, setSelectedLog] = useState<AuditLog | null>(null);

  const since = dateRange?.[0]?.toISOString();
  const until = dateRange?.[1]?.toISOString();

  const { data: logs, isLoading } = useLogs({ since, until, key_id: keyFilter, limit: 100 });
  const { data: keys } = useKeys();

  const columns = [
    { title: 'Time', dataIndex: 'created_at', key: 'created_at', width: 180,
      render: (v: string) => new Date(v).toLocaleString(),
    },
    { title: 'Model', dataIndex: 'model_name', key: 'model_name', width: 150 },
    { title: 'Protocol', dataIndex: 'protocol', key: 'protocol', width: 100,
      render: (v: string) => <Tag color={v === 'openai' ? 'blue' : 'purple'}>{v}</Tag>,
    },
    { title: 'Status', dataIndex: 'status_code', key: 'status_code', width: 80,
      render: (v: number) => <Tag color={v < 400 ? 'green' : v < 500 ? 'orange' : 'red'}>{v}</Tag>,
    },
    { title: 'Latency', dataIndex: 'latency_ms', key: 'latency_ms', width: 100,
      render: (v: number) => `${v}ms`,
    },
    { title: 'Input Tokens', dataIndex: 'input_tokens', key: 'input_tokens', width: 110,
      render: (v: number | null) => v ?? '-',
    },
    { title: 'Output Tokens', dataIndex: 'output_tokens', key: 'output_tokens', width: 110,
      render: (v: number | null) => v ?? '-',
    },
    {
      title: 'Actions', key: 'actions', width: 80,
      render: (_: unknown, record: AuditLog) => (
        <a onClick={() => setSelectedLog(record)}>View</a>
      ),
    },
  ];

  return (
    <div>
      <Title level={4}>Audit Logs</Title>

      <Card style={{ marginBottom: 16 }}>
        <Row gutter={16} align="middle">
          <Col>
            <RangePicker
              onChange={(dates) => setDateRange(dates as [Dayjs | null, Dayjs | null] | null)}
            />
          </Col>
          <Col>
            <Select
              placeholder="Filter by API Key"
              allowClear
              style={{ width: 200 }}
              onChange={(v) => setKeyFilter(v)}
              options={keys?.map(k => ({ value: k.id, label: k.name })) ?? []}
            />
          </Col>
        </Row>
      </Card>

      <Table
        dataSource={logs}
        columns={columns}
        rowKey="id"
        loading={isLoading}
        size="small"
        scroll={{ x: 1000 }}
      />

      <Drawer
        title="Log Detail"
        width={700}
        open={!!selectedLog}
        onClose={() => setSelectedLog(null)}
      >
        {selectedLog && (
          <div>
            <p><strong>Time:</strong> {new Date(selectedLog.created_at).toLocaleString()}</p>
            <p><strong>Model:</strong> {selectedLog.model_name}</p>
            <p><strong>Protocol:</strong> {selectedLog.protocol}</p>
            <p><strong>Status:</strong> <Tag color={selectedLog.status_code < 400 ? 'green' : 'red'}>{selectedLog.status_code}</Tag></p>
            <p><strong>Latency:</strong> {selectedLog.latency_ms}ms</p>
            <p><strong>Tokens:</strong> {selectedLog.input_tokens ?? 0} in / {selectedLog.output_tokens ?? 0} out</p>

            <Title level={5} style={{ marginTop: 16 }}>Request Body</Title>
            <JsonViewer data={selectedLog.request_body} />

            <Title level={5} style={{ marginTop: 16 }}>Response Body</Title>
            <JsonViewer data={selectedLog.response_body} />
          </div>
        )}
      </Drawer>
    </div>
  );
}
```

- [ ] **Step 3: Verify and commit**

Run: `cd web && npx tsc --noEmit`

```bash
git add web/src/components/JsonViewer.tsx web/src/pages/Logs.tsx
git commit -m "feat(web): implement Logs page with JSON viewer drawer"
```

---

## Task 13: Build and verify

- [ ] **Step 1: Run TypeScript check**

Run: `cd web && npx tsc --noEmit`
Expected: No errors

- [ ] **Step 2: Build for production**

Run: `cd web && npm run build`
Expected: Produces `web/dist/` with static files

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat(web): complete React management dashboard

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

---

## Summary

| Task | Description |
|---|---|
| 1 | Project scaffold (Vite + React + TS + deps) |
| 2 | TypeScript types + Axios API client |
| 3 | API functions (keys, providers, models, usage, logs) |
| 4 | React Query hooks |
| 5 | Layout + Login + Router + page placeholders |
| 6 | API Keys page (list + create modal) |
| 7 | Key Detail page (edit + delete) |
| 8 | Providers page (list + create modal) |
| 9 | Provider Detail page (edit + model management) |
| 10 | Dashboard page (stats + recent requests) |
| 11 | Usage page (filters + chart + table) |
| 12 | Logs page (filters + JSON viewer drawer) |
| 13 | Build and verify |
