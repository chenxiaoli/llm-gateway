# ChannelModel Billing Fields Design

> **Date:** 2026-04-15
> **Feature:** Add billing_type and pricing fields to ChannelModel

## Goal

Add billing_type, input_price, output_price, and request_price fields to the ChannelModel entity. These fields allow per-channel-model pricing override, giving finer control over billing at the channel-model level.

## Architecture

### Database

Add columns to `channel_models` table:
- `billing_type` VARCHAR - billing type (per_token, per_request, per_character, tiered_token, hybrid)
- `input_price` REAL - input price per 1M tokens
- `output_price` REAL - output price per 1M tokens
- `request_price` REAL - price per request

Existing fields preserved:
- `markup_ratio` (already exists, default 1.0)
- `cost_policy_id` (already exists)

### ChannelModel Struct

```rust
pub struct ChannelModel {
    pub id: String,
    pub channel_id: String,
    pub model_id: String,
    pub upstream_model_name: String,
    pub priority_override: Option<i32>,
    pub cost_policy_id: Option<String>,
    pub markup_ratio: f64,
    pub billing_type: Option<String>,   // NEW
    pub input_price: Option<f64>,       // NEW
    pub output_price: Option<f64>,      // NEW
    pub request_price: Option<f64>,     // NEW
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### Pricing Resolution Priority

When calculating cost:
1. **channel_model** has custom price (non-zero) → use channel_model price
2. **else** → fallback to model price
3. **else** → fallback to pricing_policy price

- `billing_type`: if set on channel_model, overrides model's billing_type
- `markup_ratio`: always applied as multiplier on final price

## Implementation Notes

- Migration: Add 4 columns to channel_models table (billing_type, input_price, output_price, request_price)
- Storage: Update ChannelModel struct and CRUD operations
- API: Update channel_models API handlers
- Frontend: Add fields to channel-model form