# Model-Provider N:N Relationship Redesign

**Date**: 2026-04-17
**Status**: Approved

## Summary

Redesign Model and Provider relationship from N:1 to N:N, use ChannelModel as the sole routing source, Model retains default pricing with ChannelModel for overrides.

## Background

Current architecture has Model directly linked to a single Provider (N:1), which causes issues:
- Same model on different providers (e.g., gpt-4 on OpenAI and Azure) requires duplicate Model records
- Model semantics unclear: is it "abstract model" or "concrete implementation"?

## Architecture

### Entities

| Entity | Responsibility | Example |
|--------|---------------|---------|
| **Provider** | LLM service provider | OpenAI, Azure, Anthropic |
| **Channel** | Access credentials for a Provider | API Key + Endpoint |
| **Model** | Abstract model name + default pricing | "gpt-4", $10/M input default |
| **ChannelModel** | Model ↔ Channel association + override config | Which models a Channel supports |

### Relationships

```
Model ← N:M ↔ ChannelModel ↔ N:M → Channel
         ↑
         │
      Provider (Channel belongs to a Provider)
```

### Routing (ChannelModel only)

```
Client request: model="gpt-4"
    ↓
Find all ChannelModel where model_id="gpt-4" and enabled=true
    ↓
Each ChannelModel → corresponds to a Channel
    ↓
Sort by priority, select available channel
    ↓
Get upstream_model_name and final pricing:
  - If ChannelModel has override → use it
  - Otherwise → use Model's default pricing
```

### Pricing Logic

```
Final Price = 
  ChannelModel.input_price (if not null)
  OR Model.input_price (default)

Same for output_price, request_price, billing_type.
```

## Implementation

### Database Changes

1. **Remove provider_id from Model** — Model becomes abstract, but retains default pricing
2. **No ProviderModel needed** — pricing stays on Model

### Fields

**Model** (updated):
```rust
pub struct Model {
    pub id: String,
    pub name: String,              // abstract name, e.g., "gpt-4"
    pub model_type: Option<String>,
    pub billing_type: String,      // default billing type
    pub input_price: f64,          // default price
    pub output_price: f64,
    pub request_price: f64,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    // Removed: provider_id, pricing_policy_id
}
```

**ChannelModel** (existing, enhanced):
```rust
pub struct ChannelModel {
    pub id: String,
    pub channel_id: String,
    pub model_id: String,
    pub upstream_model_name: String,
    pub priority_override: Option<i32>,
    pub markup_ratio: f64,             // default 1.0
    pub billing_type: Option<String>,  // override
    pub input_price: Option<f64>,      // override
    pub output_price: Option<f64>,     // override
    pub request_price: Option<f64>,    // override
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### Routing Code

```rust
// Get pricing: override > default
let final_input_price = channel_model.input_price
    .unwrap_or(model.input_price);
```

### Migration Strategy

1. Remove provider_id from Model (data migration: set to null)
2. Update routing code to use ChannelModel only
3. No ProviderModel needed

### Frontend Impact

**Channel Edit Flow**:
1. Fetch all enabled Models
2. User selects models to enable → create ChannelModel records
3. Optionally override pricing per ChannelModel

## Backward Compatibility

- Requires database migration
- API responses change (Model no longer has provider_id)
- Frontend needs updates to support new flow

## Testing

- Unit test: routing uses ChannelModel only
- Unit test: ChannelModel query filters by model_id and enabled
- Integration test: end-to-end request routing with new schema