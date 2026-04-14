# Provider Endpoints Migration Design

## Overview

Simplify provider configuration by replacing separate `openai_base_url` and `anthropic_base_url` fields with a unified `endpoints` JSON field.

## Changes

### Database Schema

**Remove:**
- `openai_base_url` (TEXT)
- `anthropic_base_url` (TEXT)

**Add:**
- `base_url` (TEXT, nullable) - Single fallback URL for display/legacy
- `endpoints` (TEXT, nullable) - JSON: `{"openai": "https://...", "anthropic": "https://..."}`

### Storage Layer

**Files:**
- `crates/storage/src/types.rs` - Update Provider struct
- `crates/storage/src/sqlite.rs` - Update migrations and queries
- `crates/storage/migrations/` - Add migration

### API Layer

**Files:**
- `crates/api/src/management/providers.rs` - Update request/response types

### Gateway Layer

**Files:**
- `crates/api/src/openai.rs` - Read from `endpoints["openai"]`
- `crates/api/src/anthropic.rs` - Read from `endpoints["anthropic"]`

### Frontend

**Files:**
- `web/src/types/index.ts` - Update Provider interface
- `web/src/pages/Settings.tsx` - Update provider form

## Data Flow

```
Request → /v1/chat/completions (OpenAI)
       → Handler knows protocol is "openai"
       → Lookup provider.endpoints["openai"]
       → Use URL for upstream request
```

## Backward Compatibility

- Add migration that converts existing `openai_base_url`/`anthropic_base_url` to `endpoints` JSON
- Handle null endpoints gracefully in gateway code