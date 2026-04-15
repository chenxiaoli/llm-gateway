# Pricing Policies Design Spec

**Date:** 2026-04-15

## Overview

Refactor billing from hardcoded enum to strategy pattern using `pricing_policies` table. This enables adding new billing types without schema changes.

## Background

Current architecture has `billing_type` enum on models:
- `billing_type` = token | request
- Adding new billing types requires code changes and migrations

New design:
- `pricing_policies` table stores billing strategy as JSON config
- Three-level fallback chain for policy resolution
- Cost/price separation via `cost_policy_id` + `markup_ratio`

## Data Model

### Table: pricing_policies

```sql
CREATE TABLE pricing_policies (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,           -- human-readable name
    billing_type TEXT NOT NULL,         -- per_token, per_request, per_character, tiered_token, hybrid
    config      TEXT NOT NULL,          -- JSON config per billing_type
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE INDEX idx_pricing_policies_type ON pricing_policies(billing_type);
```

### Billing Types and Configs

```json
// per_token: OpenAI, Anthropic mainstream
{ "billing_type": "per_token", "config": { "input_per_1k": 0.003, "output_per_1k": 0.015 } }

// per_request: certain tool APIs, image generation
{ "billing_type": "per_request", "config": { "price_per_call": 0.002 } }

// per_character: some domestic vendors (Baidu, etc)
{ "billing_type": "per_character", "config": { "input_per_1k": 0.001, "output_per_1k": 0.004 } }

// tiered_token: volume discount
{ "billing_type": "tiered_token", "config": { "tiers": [
    { "up_to": 1000000, "input_per_1k": 0.005, "output_per_1k": 0.015 },
    { "up_to": null,    "input_per_1k": 0.004, "output_per_1k": 0.012 }
]}}

// hybrid: base request fee + usage
{ "billing_type": "hybrid", "config": { "base_per_call": 0.0005, "input_per_1k": 0.001, "output_per_1k": 0.003 } }
```

### Model: Add pricing_policy_id

```sql
ALTER TABLE models ADD COLUMN pricing_policy_id TEXT REFERENCES pricing_policies(id);
```

### Channel: Add pricing_policy_id + markup_ratio

```sql
ALTER TABLE channels ADD COLUMN pricing_policy_id TEXT REFERENCES pricing_policies(id);
ALTER TABLE channels ADD COLUMN markup_ratio REAL NOT NULL DEFAULT 1.0;
```

### ChannelModel: Add cost_policy_id + markup_ratio

```sql
ALTER TABLE channel_models ADD COLUMN cost_policy_id TEXT REFERENCES pricing_policies(id);
ALTER TABLE channel_models ADD COLUMN markup_ratio REAL NOT NULL DEFAULT 1.0;
```

## Policy Resolution Chain

Three-level fallback for cost policy:

```
channel_models.cost_policy_id → channel.pricing_policy_id → model.pricing_policy_id
```

Price calculation:
```
price = cost * markup_ratio
```

Resolution query:
```sql
SELECT 
    COALESCE(cm.cost_policy_id, c.pricing_policy_id, m.pricing_policy_id) AS resolved_cost_policy_id,
    cm.markup_ratio * c.markup_ratio AS resolved_markup
FROM channel_models cm
JOIN channels c ON c.id = cm.channel_id
JOIN models m ON m.id = cm.model_id
WHERE cm.channel_id = $1 AND m.name = $2;
```

## Rust Structs

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingPolicy {
    pub id: String,
    pub name: String,
    pub billing_type: String,           // "per_token", "per_request", etc.
    pub config: serde_json::Value,        // billing-type-specific config
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePricingPolicy {
    pub name: String,
    pub billing_type: String,
    pub config: serde_json::Value,
}

// Model: remove billing_type, add pricing_policy_id
pub struct Model {
    pub id: String,
    pub name: String,
    pub provider_id: String,
    pub model_type: Option<String>,
    pub pricing_policy_id: Option<String>,  // NEW
    pub input_price: f64,
    pub output_price: f64,
    pub request_price: f64,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

// Channel: add pricing_policy_id + markup_ratio
pub struct Channel {
    pub id: String,
    pub provider_id: String,
    pub name: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub priority: i32,
    pub pricing_policy_id: Option<String>,  // NEW
    pub markup_ratio: f64,                    // NEW, default 1.0
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ChannelModel: add cost_policy_id + markup_ratio
pub struct ChannelModel {
    pub id: String,
    pub channel_id: String,
    pub model_id: String,
    pub upstream_model_name: String,
    pub priority_override: Option<i32>,
    pub cost_policy_id: Option<String>,   // NEW
    pub markup_ratio: f64,                 // NEW, default 1.0
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

## Pricing Calculator

Application-layer calculation (not in DB):

```rust
pub struct Usage {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub input_chars: Option<i64>,
    pub output_chars: Option<i64>,
    pub request_count: i64,
}

pub struct PricingCalculator;

impl PricingCalculator {
    pub fn calculate_cost(&self, policy: &PricingPolicy, usage: &Usage) -> Decimal {
        let cfg = &policy.config;
        match policy.billing_type.as_str() {
            "per_token" => {
                let input = (usage.input_tokens as f64 / 1000.0) 
                    * cfg["input_per_1k"].as_f64().unwrap_or(0.0);
                let output = (usage.output_tokens as f64 / 1000.0) 
                    * cfg["output_per_1k"].as_f64().unwrap_or(0.0);
                Decimal::from(input + output)
            }
            "per_request" => {
                Decimal::from(usage.request_count as f64 
                    * cfg["price_per_call"].as_f64().unwrap_or(0.0))
            }
            "per_character" => {
                let input = (usage.input_chars.unwrap_or(0) as f64 / 1000.0) 
                    * cfg["input_per_1k"].as_f64().unwrap_or(0.0);
                let output = (usage.output_chars.unwrap_or(0) as f64 / 1000.0) 
                    * cfg["output_per_1k"].as_f64().unwrap_or(0.0);
                Decimal::from(input + output)
            }
            "tiered_token" => self.calculate_tiered(cfg, usage),
            "hybrid" => {
                let base = Decimal::from(usage.request_count as f64 
                    * cfg["base_per_call"].as_f64().unwrap_or(0.0));
                let usage_cost = /* per_token calc */;
                base + usage_cost
            }
            _ => Decimal::ZERO,
        }
    }

    fn calculate_tiered(&self, config: &Value, usage: &Usage) -> Decimal {
        // tiered logic: apply tier pricing based on volume
        let tiers = config["tiers"].as_array().unwrap();
        // ... implementation
    }
}
```

## Storage Layer

New trait methods:

```rust
trait Storage {
    // ... existing methods ...

    // Pricing Policies
    async fn create_pricing_policy(&self, p: &PricingPolicy) -> Result<PricingPolicy, DbErr>;
    async fn get_pricing_policy(&self, id: &str) -> Result<Option<PricingPolicy>, DbErr>;
    async fn list_pricing_policies(&self) -> Result<Vec<PricingPolicy>, DbErr>;
    async fn delete_pricing_policy(&self, id: &str) -> Result<(), DbErr>;

    // Model: update to use pricing_policy_id instead of billing_type
    async fn update_model(&self, m: &Model) -> Result<Model, DbErr>;
    async fn create_model(&self, m: &Model) -> Result<Model, DbErr>;

    // Channel: add pricing_policy_id, markup_ratio
    async fn update_channel(&self, c: &Channel) -> Result<Channel, DbErr>;
    async fn create_channel(&self, c: &Channel) -> Result<Channel, DbErr>;

    // ChannelModel: add cost_policy_id, markup_ratio
    async fn update_channel_model(&self, cm: &ChannelModel) -> Result<ChannelModel, DbErr>;
    async fn create_channel_model(&self, cm: &ChannelModel) -> Result<ChannelModel, DbErr>;
}
```

## API Endpoints

### Create Pricing Policy
```
POST /management/pricing-policies
{
    "name": "GPT-4o Standard",
    "billing_type": "per_token",
    "config": { "input_per_1k": 0.005, "output_per_1k": 0.015 }
}
```

### List Pricing Policies
```
GET /management/pricing-policies
```

### Get Pricing Policy
```
GET /management/pricing-policies/{id}
```

### Delete Pricing Policy
```
DELETE /management/pricing-policies/{id}
```

### Update Model (pricing_policy_id)
```
PATCH /management/models/{id}
{
    "pricing_policy_id": "uuid-or-null"
}
```

### Update Channel (pricing_policy_id, markup_ratio)
```
PATCH /management/channels/{id}
{
    "pricing_policy_id": "uuid-or-null",
    "markup_ratio": 1.2
}
```

### Update ChannelModel (cost_policy_id, markup_ratio)
```
PATCH /management/channel-models/{id}
{
    "cost_policy_id": "uuid-or-null",
    "markup_ratio": 1.1
}
```

## Migration

### Step 1: Create pricing_policies table

```sql
CREATE TABLE pricing_policies (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    billing_type TEXT NOT NULL,
    config      TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE INDEX idx_pricing_policies_type ON pricing_policies(billing_type);
```

### Step 2: Migrate existing models to policies

```sql
-- Generate policy for each model based on its billing_type
INSERT INTO pricing_policies (id, name, billing_type, config, created_at, updated_at)
SELECT 
    'policy-' || id,
    name || ' Policy',
    CASE billing_type 
        WHEN 'token' THEN 'per_token'
        WHEN 'request' THEN 'per_request'
        ELSE 'per_token'
    END,
    CASE billing_type 
        WHEN 'token' THEN json_object('input_per_1k', input_price, 'output_per_1k', output_price)
        WHEN 'request' THEN json_object('price_per_call', request_price)
        ELSE json_object('input_per_1k', 0, 'output_per_1k', 0)
    END,
    created_at,
    created_at
FROM models;

-- Link models to their policies
UPDATE models SET pricing_policy_id = 'policy-' || id WHERE pricing_policy_id IS NULL;
```

### Step 3: Add columns to channels, channel_models

```sql
ALTER TABLE channels ADD COLUMN pricing_policy_id TEXT REFERENCES pricing_policies(id);
ALTER TABLE channels ADD COLUMN markup_ratio REAL NOT NULL DEFAULT 1.0;

ALTER TABLE channel_models ADD COLUMN cost_policy_id TEXT REFERENCES pricing_policies(id);
ALTER TABLE channel_models ADD COLUMN markup_ratio REAL NOT NULL DEFAULT 1.0;
```

### Step 4: Remove billing_type from models

```sql
-- SQLite: recreate table
CREATE TABLE models_new AS SELECT id, name, provider_id, model_type, pricing_policy_id, 
    input_price, output_price, request_price, enabled, created_at FROM models;
DROP TABLE models;
ALTER TABLE models_new RENAME TO models;
```

## Testing

1. **Create policy**: Create per_token policy, verify in DB
2. **Policy resolution**: Model → Channel → ChannelModel fallback works
3. **Cost calculation**: PricingCalculator produces correct cost for each billing_type
4. **Price with markup**: cost * markup_ratio = price
5. **Migration**: Old billing_type data correctly converted to policies
6. **API**: CRUD operations work for pricing_policies
7. **Null fallback**: No policy set → returns zero cost (safe default)

## Out of Scope

- Frontend UI for policy management (future iteration)
- Price policy separate from cost policy (use markup_ratio for now)
- BillingType enum removal from Rust (keep in types.rs for migration compatibility)