# Model billing_type Field Design

> **Date:** 2026-04-15
> **Feature:** Add billing_type to Model

## Goal

Add `billing_type` field to the Model entity to support different billing strategies directly on the model level. This allows per-model billing type configuration without requiring a pricing policy.

## Architecture

### Database

- Add `billing_type` column to `models` table
- Type: VARCHAR
- Default: "per_token"

### Supported billing_type Values

| Value | Description |
|-------|-------------|
| `per_token` | Per-token billing (default) |
| `per_request` | Per-request billing |
| `per_character` | Per-character billing |
| `tiered_token` | Tiered token billing |
| `hybrid` | Hybrid billing |

### Model Struct

```rust
pub struct Model {
    pub id: String,
    pub name: String,
    pub provider_id: String,
    pub model_type: Option<String>,
    pub pricing_policy_id: Option<String>,
    pub billing_type: String,  // NEW
    pub input_price: f64,
    pub output_price: f64,
    pub request_price: f64,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}
```

## Implementation Notes

- Storage layer: Add billing_type to Model struct and SQLite operations
- API layer: Update model CRUD handlers to accept/return billing_type
- Frontend: Add billing_type dropdown to model form and list view
- Migration: Add column with default value (no data migration needed)