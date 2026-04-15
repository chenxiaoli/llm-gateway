# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

LLM Gateway is a unified API gateway for LLM providers with OpenAI (`/v1/chat/completions`) and Anthropic (`/v1/messages`) compatible endpoints. Built as a Rust workspace with a React frontend in `web/`.

## Commands

### Backend (Rust)

```bash
cargo test --workspace        # Run all tests
cargo build --release         # Build release binary
cargo run                      # Run dev server on :8080
```

### Frontend (web/)

```bash
cd web
npm run dev                    # Dev server on :5173 with API proxy to :8080
npm run build                  # TypeScript check + Vite build → dist/
npm test                       # Vitest unit tests
npm test -- src/api/keys.test.ts  # Run single test file
npm run test:e2e               # Playwright e2e tests
npm run test:e2e:ui            # Playwright e2e with UI
```

## Architecture

### Backend (Rust workspace)

```
crates/
├── gateway/      # Server bootstrap, config, assembles all modules
├── api/          # HTTP handlers (Axum) — openai.rs, anthropic.rs, management.rs, middleware.rs
├── storage/      # SQLite/PostgreSQL storage trait + migrations
├── auth/         # API key generation and SHA-256 verification
├── ratelimit/    # In-memory sliding window rate limiter
├── billing/      # Token/request cost calculation
├── audit/        # Request audit logging
├── provider/     # Provider trait and upstream proxy (OpenAI + Anthropic adapters)
└── encryption/   # API key encryption at rest
```

**Request flow**: Auth (API key) → Rate Limit → Router (model → provider + protocol) → Upstream Proxy → Response (streaming SSE or buffered) → async audit + billing write.

### Frontend (React + TypeScript)

```
web/src/
├── api/          # API client (axios) + endpoint functions
├── components/
│   ├── ui/       # Custom UI primitives (Button, Modal, Drawer, Badge, Toggle, Select, etc.)
│   └── Layout.tsx # Sidebar layout with React Router
├── hooks/        # React Query hooks for server state
├── stores/       # Zustand store (auth only — useAuthStore)
├── pages/        # Route pages (Dashboard, Keys, Providers, Channels, Models, Users, Settings, Usage, Logs)
├── types/        # TypeScript types
└── lib/          # Utilities (cn.ts — tailwind-merge helper)
```

**State architecture**:
- Zustand: auth state only (`useAuthStore`)
- React Query: all server state (keys, providers, usage, logs)
- React useState: local UI state

**API client** (`web/src/api/client.ts`): Axios with `/api/v1` base URL, Bearer token in requests, automatic refresh token handling on 401.

### Design System

Dark theme, ultra-clean minimal aesthetic (Linear/Vercel inspired). Currently migrating from Ant Design to Tailwind CSS + custom components. Uses `sonner` for toasts, `lucide-react` for icons, `@tanstack/react-table` for tables, `react-hook-form` + `zod` for forms.

## Configuration

Backend config via `config.toml` in the working directory (server, admin token, database, rate limit, upstream timeout, audit retention). Business data (keys, providers, models) managed via Management API or web UI.

## Development Workflow

1. Run backend: `cargo run` (port 8080)
2. Run frontend: `cd web && npm run dev` (port 5173, proxies `/api` to 8080)
3. Frontend hot-reloads independently; API calls route to backend in dev
