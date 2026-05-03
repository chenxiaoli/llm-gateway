# Drop SQLite Support — Design Spec

**Date:** 2026-05-03
**Status:** Approved

## Goal

Remove all SQLite code from the codebase starting at v1.0.0. PostgreSQL is the only supported database driver. Simplifies the storage layer, removes ~2250 lines of duplicate implementation and 20 migration files, eliminates `libsqlite3-sys` native dependency from builds.

## Files Deleted

| Path | Description |
|---|---|
| `crates/storage/src/sqlite.rs` | Full SQLite Storage trait implementation (~2250 lines) |
| `crates/storage/migrations/sqlite/*.sql` | 20 SQLite migration files |

## Files Modified

### `crates/storage/Cargo.toml`
- Remove `default = ["sqlite"]` and `sqlite = []` feature flags
- Only `postgres = []` feature remains (or remove features entirely if postgres is always enabled)

### `crates/storage/src/lib.rs`
- Remove `#[cfg(feature = "sqlite")] pub mod sqlite;`

### `crates/storage/build.rs`
- Remove `cargo:rerun-if-changed=migrations/sqlite/*` line

### `Cargo.toml` (workspace root)
- Remove `"sqlite"` from sqlx features array

### `crates/gateway/Cargo.toml`
- Remove `"sqlite"` from `llm-gateway-storage` features

### `crates/gateway/src/main.rs`
- Remove `"sqlite"` match arm from driver selection
- Remove `SqliteStorage` import
- Driver validation only accepts `"postgres"`, everything else is an error
- Remove `sqlite` from error message listing supported drivers

### `crates/api/tests/common/mod.rs`
- Replace `SqliteStorage::new_in_memory()` with PostgreSQL test helper
- Connect via `DATABASE_URL` env var (required for tests)
- Run migrations on connect
- Provide table truncation for test isolation

### `crates/api/tests/test_*.rs` (5 files)
- Update `make_state()` signatures from `Arc<SqliteStorage>` to `Arc<PostgresStorage>`
- Remove `"sqlite"` driver string, use `"postgres"`

### `README.md`
- Remove SQLite references from architecture diagram, config examples, storage description

### `CLAUDE.md`
- Update storage description from "SQLite/PostgreSQL" to "PostgreSQL"

### `CHANGELOG.md`
- Add entry under v1.0.0 noting SQLite removal

### `web/src/test/server.ts`
- Change `database_driver: 'sqlite'` to `'postgres'`

### `.github/workflows/ci.yml`
- Update `DATABASE_URL=sqlite:data/gateway.db` to PostgreSQL test URL for `cargo sqlx prepare`

## Test Migration Strategy

All 5 integration test files currently use `SqliteStorage::new_in_memory()` for isolation. These will be migrated to:

1. A `setup_test_db()` function that connects to PostgreSQL via `DATABASE_URL` env var
2. Returns `Arc<PostgresStorage>` with migrations applied
3. Test isolation via table truncation between tests (or unique schema per test run)

Tests require `DATABASE_URL` env var to be set pointing at a PostgreSQL instance.

## Not Changed

- PostgreSQL storage implementation (`postgres.rs`)
- Storage trait (`lib.rs` trait definition)
- Any business logic, billing, audit, NATS code
- Frontend code (except test mock)
- Config format (already postgres-only in production)

## Scope

- Purely mechanical removal — no schema changes, no new features
- No behavior changes for PostgreSQL users
- Single commit on release branch
