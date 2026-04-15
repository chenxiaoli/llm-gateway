# PostgreSQL Support Design

> **Date:** 2026-04-15
> **Feature:** Add PostgreSQL Support

## Goal

Add PostgreSQL database support to llm-gateway by abstracting the storage layer and implementing both SQLite and PostgreSQL backends.

## Architecture

```
┌─────────────────────────────────────┐
│           API Layer                 │
│    (crates/api/src/management/*)   │
└──────────────┬──────────────────────┘
               │
               ▼
┌─────────────────────────────────────┐
│        Storage Trait                │
│     (crates/storage/src/lib.rs)     │
└──────────────┬──────────────────────┘
               │
       ┌───────┴───────┐
       ▼               ▼
┌─────────────┐   ┌─────────────┐
│ SqliteStorage│   │PgStorage    │
│ (sqlite)     │   │(postgres)   │
└─────────────┘   └─────────────┘
```

## Feature Flags

```toml
# crates/storage/Cargo.toml
[features]
default = ["sqlite"]
sqlite = []
postgres = []
```

## Configuration

Database type is selected via config.yaml:

```yaml
database:
  type: "sqlite"  # or "postgres"
  
  # For SQLite:
  path: "data.db"
  
  # For PostgreSQL:
  host: "your-remote-host.com"
  port: 5432
  user: "user"
  password: "pass"
  database: "llm_gateway"
```

## Migrations

Migrations organized by database type:

```
migrations/
├── sqlite/       # SQLite migrations
└── postgres/     # PostgreSQL migrations
```

Key differences:
- PostgreSQL uses `gen_ulid()` for ID generation (not `gen_random_uuid()`)
- PostgreSQL uses `TIMESTAMP WITH TIME ZONE`
- Type differences handled in separate migration files

## Implementation Plan

1. Create `Storage` trait in `lib.rs`
2. Refactor `sqlite.rs` to implement `Storage` trait
3. Create `postgres.rs` implementing `Storage` trait
4. Update `lib.rs` to export storage based on feature flags
5. Add migrations for PostgreSQL in `migrations/postgres/`
6. Update configuration handling

## Database Tables (Same as SQLite)

- api_keys
- providers
- channels
- models
- channel_models
- key_model_rate_limits
- usage_records
- audit_logs
- users
- refresh_tokens
- pricing_policies
- schema_migrations

All tables maintain compatible schema across both databases.