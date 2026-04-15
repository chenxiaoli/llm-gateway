# Pricing Policies Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor billing from hardcoded enum to strategy pattern using pricing_policies table, enabling new billing types without schema changes.

**Architecture:** Add pricing_policies table with JSON config, three-level fallback chain (channel_models → channel → model) for policy resolution, cost/price separation via cost_policy_id + markup_ratio.

**Tech Stack:** Rust, SQLite, serde_json

---

## File Structure

- Create: `crates/billing/src/pricing.rs` (PricingCalculator)
- Modify: `crates/storage/src/types.rs:140-180` (Model, BillingType, CreateModel, UpdateModel)
- Modify: `crates/storage/src/types.rs:96-138` (Channel, CreateChannel, UpdateChannel)
- Modify: `crates/storage/src/types.rs:189-216` (ChannelModel, CreateChannelModel, UpdateChannelModel)
- Modify: `crates/storage/src/lib.rs:7-90` (Storage trait - add pricing policy methods)
- Modify: `crates/storage/src/sqlite.rs` (implement Storage trait methods)
- Modify: `crates/billing/src/lib.rs` (remove old calculate_cost, use PricingCalculator)
- Modify: `crates/api/src/openai.rs` (use PricingCalculator)
- Modify: `crates/api/src/anthropic.rs` (use PricingCalculator)
- Create: `crates/storage/migrations/20260415000000_pricing_policies.sql`
- Create: `crates/api/src/management/pricing_policies.rs` (API handlers)

---

## Task 1: Add PricingPolicy types to storage

**Files:**
- Modify: `crates/storage/src/types.rs`

- [ ] **Step 1: Add PricingPolicy struct after BillingType enum (around line 161)**

Add these types after the existing BillingType enum:

```rust
// --- Pricing Policies ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingPolicy {
    pub id: String,
    pub name: String,
    pub billing_type: String,        // "per_token", "per_request", "per_character", "tiered_token", "hybrid"
    pub config: serde_json::Value,   // billing-type-specific config
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePricingPolicy {
    pub name: String,
    pub billing_type: String,
    pub config: serde_json::Value,
}
```

- [ ] **Step 2: Add Usage struct for calculator**

Add after PricingPolicy:

```rust
// --- Usage for pricing calculation ---

#[derive(Debug, Clone, Copy)]
pub struct Usage {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub input_chars: Option<i64>,
    pub output_chars: Option<i64>,
    pub request_count: i64,
}

impl Usage {
    pub fn from_tokens(input: Option<i64>, output: Option<i64>, requests: i64) -> Self {
        Usage {
            input_tokens: input.unwrap_or(0),
            output_tokens: output.unwrap_or(0),
            input_chars: None,
            output_chars: None,
            request_count: requests,
        }
    }
}
```

- [ ] **Step 3: Commit**

```bash
git add crates/storage/src/types.rs
git commit -m "feat: add PricingPolicy and Usage types"
```

---

## Task 2: Modify Model to remove billing_type, add pricing_policy_id

**Files:**
- Modify: `crates/storage/src/types.rs:142-179`

- [ ] **Step 1: Replace Model struct (lines 142-154)**

Replace current Model with:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub id: String,
    pub name: String,
    pub provider_id: String,
    pub model_type: Option<String>,
    pub pricing_policy_id: Option<String>,  // NEW: nullable FK to pricing_policies
    pub input_price: f64,
    pub output_price: f64,
    pub request_price: f64,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}
```

- [ ] **Step 2: Update CreateModel (lines 163-170)**

Replace with:

```rust
#[derive(Debug, Deserialize)]
pub struct CreateModel {
    pub name: String,
    pub pricing_policy_id: Option<String>,  // NEW
    pub input_price: Option<f64>,
    pub output_price: Option<f64>,
    pub request_price: Option<f64>,
}
```

- [ ] **Step 3: Update UpdateModel (lines 172-179)**

Replace with:

```rust
#[derive(Debug, Deserialize)]
pub struct UpdateModel {
    pub pricing_policy_id: Option<Option<String>>,  // None=keep, Some(None)=clear
    pub input_price: Option<f64>,
    pub output_price: Option<f64>,
    pub request_price: Option<f64>,
    pub enabled: Option<bool>,
}
```

- [ ] **Step 4: Remove BillingType from public exports (keep for migration compat)**

Note: Keep BillingType in types.rs but mark as deprecated in comments. Don't export from lib.rs.

- [ ] **Step 5: Commit**

```bash
git add crates/storage/src/types.rs
git commit -m "refactor: replace Model.billing_type with pricing_policy_id"
```

---

## Task 3: Modify Channel to add pricing_policy_id and markup_ratio

**Files:**
- Modify: `crates/storage/src/types.rs:98-138`

- [ ] **Step 1: Update Channel struct (lines 98-113)**

Replace with:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub provider_id: String,
    pub name: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub priority: i32,
    pub pricing_policy_id: Option<String>,  // NEW
    pub markup_ratio: f64,                  // NEW, default 1.0
    pub rpm_limit: Option<i64>,
    pub tpm_limit: Option<i64>,
    pub balance: Option<f64>,
    pub weight: Option<i32>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

- [ ] **Step 2: Update CreateChannel (lines 115-125)**

Replace with:

```rust
#[derive(Debug, Deserialize)]
pub struct CreateChannel {
    pub name: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub priority: Option<i32>,
    pub pricing_policy_id: Option<String>,  // NEW
    pub markup_ratio: Option<f64>,          // NEW
    pub rpm_limit: Option<i64>,
    pub tpm_limit: Option<i64>,
    pub balance: Option<f64>,
    pub weight: Option<i32>,
}
```

- [ ] **Step 3: Update UpdateChannel (lines 127-138)**

Replace with:

```rust
#[derive(Debug, Deserialize)]
pub struct UpdateChannel {
    pub name: Option<String>,
    pub api_key: Option<String>,
    pub base_url: Option<Option<String>>,
    pub priority: Option<i32>,
    pub pricing_policy_id: Option<Option<String>>,  // NEW
    pub markup_ratio: Option<f64>,                  // NEW
    pub enabled: Option<bool>,
    pub rpm_limit: Option<Option<i64>>,
    pub tpm_limit: Option<Option<i64>>,
    pub balance: Option<Option<f64>>,
    pub weight: Option<Option<i32>>,
}
```

- [ ] **Step 4: Commit**

```bash
git add crates/storage/src/types.rs
git commit -m "feat: add pricing_policy_id and markup_ratio to Channel"
```

---

## Task 4: Modify ChannelModel to add cost_policy_id and markup_ratio

**Files:**
- Modify: `crates/storage/src/types.rs:189-216`

- [ ] **Step 1: Update ChannelModel struct (lines 191-201)**

Replace with:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelModel {
    pub id: String,
    pub channel_id: String,
    pub model_id: String,
    pub upstream_model_name: String,
    pub priority_override: Option<i32>,
    pub cost_policy_id: Option<String>,   // NEW: for upstream cost
    pub markup_ratio: f64,                  // NEW, default 1.0
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

- [ ] **Step 2: Update CreateChannelModel (lines 203-209)**

Replace with:

```rust
#[derive(Debug, Deserialize)]
pub struct CreateChannelModel {
    pub channel_id: String,
    pub model_id: String,
    pub upstream_model_name: String,
    pub priority_override: Option<i32>,
    pub cost_policy_id: Option<String>,   // NEW
    pub markup_ratio: Option<f64>,         // NEW
}
```

- [ ] **Step 3: Update UpdateChannelModel (lines 211-216)**

Replace with:

```rust
#[derive(Debug, Deserialize)]
pub struct UpdateChannelModel {
    pub upstream_model_name: Option<String>,
    pub priority_override: Option<Option<i32>>,
    pub cost_policy_id: Option<Option<String>>,  // NEW
    pub markup_ratio: Option<f64>,                // NEW
    pub enabled: Option<bool>,
}
```

- [ ] **Step 4: Commit**

```bash
git add crates/storage/src/types.rs
git commit -m "feat: add cost_policy_id and markup_ratio to ChannelModel"
```

---

## Task 5: Add Storage trait methods for pricing policies

**Files:**
- Modify: `crates/storage/src/lib.rs`

- [ ] **Step 1: Add pricing policy methods to Storage trait (after line 25)**

```rust
// Pricing Policies
async fn create_pricing_policy(&self, policy: &PricingPolicy) -> Result<PricingPolicy, Box<dyn std::error::Error + Send + Sync>>;
async fn get_pricing_policy(&self, id: &str) -> Result<Option<PricingPolicy>, Box<dyn std::error::Error + Send + Sync>>;
async fn list_pricing_policies(&self) -> Result<Vec<PricingPolicy>, Box<dyn std::error::Error + Send + Sync>>;
async fn update_pricing_policy(&self, policy: &PricingPolicy) -> Result<PricingPolicy, Box<dyn std::error::Error + Send + Sync>>;
async fn delete_pricing_policy(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
```

- [ ] **Step 2: Commit**

```bash
git add crates/storage/src/lib.rs
git commit -m "feat: add Storage trait methods for pricing policies"
```

---

## Task 6: Create migration SQL

**Files:**
- Create: `crates/storage/migrations/20260415000000_pricing_policies.sql`

- [ ] **Step 1: Write migration**

```sql
-- Step 1: Create pricing_policies table
CREATE TABLE IF NOT EXISTS pricing_policies (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    billing_type TEXT NOT NULL,
    config      TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_pricing_policies_type ON pricing_policies(billing_type);

-- Step 2: Add columns to channels
ALTER TABLE channels ADD COLUMN pricing_policy_id TEXT REFERENCES pricing_policies(id);
ALTER TABLE channels ADD COLUMN markup_ratio REAL NOT NULL DEFAULT 1.0;

-- Step 3: Add columns to channel_models
ALTER TABLE channel_models ADD COLUMN cost_policy_id TEXT REFERENCES pricing_policies(id);
ALTER TABLE channel_models ADD COLUMN markup_ratio REAL NOT NULL DEFAULT 1.0;

-- Step 4: Add column to models (pricing_policy_id)
ALTER TABLE models ADD COLUMN pricing_policy_id TEXT REFERENCES pricing_policies(id);

-- Step 5: Migrate existing model billing data to policies
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
FROM models WHERE billing_type IS NOT NULL;

-- Step 6: Link models to their policies
UPDATE models SET pricing_policy_id = 'policy-' || id WHERE pricing_policy_id IS NULL;

-- Step 7: Remove billing_type from models (SQLite requires recreate)
CREATE TABLE models_new AS SELECT 
    id, name, provider_id, model_type, pricing_policy_id, 
    input_price, output_price, request_price, enabled, created_at 
FROM models;
DROP TABLE models;
ALTER TABLE models_new RENAME TO models;
```

- [ ] **Step 2: Commit**

```bash
git add crates/storage/migrations/20260415000000_pricing_policies.sql
git commit -m "feat: add pricing policies migration"
```

---

## Task 7: Implement Storage trait methods in sqlite.rs

**Files:**
- Modify: `crates/storage/src/sqlite.rs`

- [ ] **Step 1: Add pricing policy CRUD functions**

Add after the existing model functions (around line 800):

```rust
// --- Pricing Policies ---

pub async fn create_pricing_policy(
    &self,
    policy: &PricingPolicy,
) -> Result<PricingPolicy, Box<dyn std::error::Error + Send + Sync>> {
    let now = Utc::now().to_rfc3339();
    self.execute(
        "INSERT INTO pricing_policies (id, name, billing_type, config, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
        rusqlite::params![
            policy.id,
            policy.name,
            policy.billing_type,
            policy.config.to_string(),
            now,
            now
        ],
    )
    .await?;
    Ok(policy.clone())
}

pub async fn get_pricing_policy(
    &self,
    id: &str,
) -> Result<Option<PricingPolicy>, Box<dyn std::error::Error + Send + Sync>> {
    let mut stmt = self
        .prepare("SELECT id, name, billing_type, config, created_at, updated_at FROM pricing_policies WHERE id = ?")
        .await?;
    let result = stmt
        .query_row(rusqlite::params![id], |row| {
            Ok(PricingPolicy {
                id: row.get(0)?,
                name: row.get(1)?,
                billing_type: row.get(2)?,
                config: serde_json::from_str(&row.get::<_, String>(3)?).unwrap_or(serde_json::Value::Null),
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })
        .await;
    match result {
        Ok(p) => Ok(Some(p)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(Box::new(e)),
    }
}

pub async fn list_pricing_policies(
    &self,
) -> Result<Vec<PricingPolicy>, Box<dyn std::error::Error + Send + Sync>> {
    let mut stmt = self
        .prepare("SELECT id, name, billing_type, config, created_at, updated_at FROM pricing_policies")
        .await?;
    let rows = stmt
        .query_map(rusqlite::params![], |row| {
            Ok(PricingPolicy {
                id: row.get(0)?,
                name: row.get(1)?,
                billing_type: row.get(2)?,
                config: serde_json::from_str(&row.get::<_, String>(3)?).unwrap_or(serde_json::Value::Null),
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })
        .await?;
    let mut policies = Vec::new();
    for row in rows {
        policies.push(row?);
    }
    Ok(policies)
}

pub async fn update_pricing_policy(
    &self,
    policy: &PricingPolicy,
) -> Result<PricingPolicy, Box<dyn std::error::Error + Send + Sync>> {
    let now = Utc::now().to_rfc3339();
    self.execute(
        "UPDATE pricing_policies SET name = ?, billing_type = ?, config = ?, updated_at = ? WHERE id = ?",
        rusqlite::params![
            policy.name,
            policy.billing_type,
            policy.config.to_string(),
            now,
            policy.id
        ],
    )
    .await?;
    Ok(policy.clone())
}

pub async fn delete_pricing_policy(
    &self,
    id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    self.execute(
        "DELETE FROM pricing_policies WHERE id = ?",
        rusqlite::params![id],
    )
    .await?;
    Ok(())
}
```

- [ ] **Step 2: Add pricing_policy_id to Model queries**

Find model insert (line 748) and update (line 820) functions, add pricing_policy_id column.

- [ ] **Step 3: Add pricing_policy_id and markup_ratio to Channel queries**

Find channel insert and update functions, add these columns.

- [ ] **Step 4: Add cost_policy_id and markup_ratio to ChannelModel queries**

Find channel_model insert and update functions, add these columns.

- [ ] **Step 5: Implement trait methods**

Add the Storage trait implementation for pricing policies (use blanket impl or add to SqliteStorage).

- [ ] **Step 6: Commit**

```bash
git add crates/storage/src/sqlite.rs
git commit -m "feat: implement pricing policy storage methods"
```

---

## Task 8: Create PricingCalculator in billing crate

**Files:**
- Create: `crates/billing/src/pricing.rs`

- [ ] **Step 1: Write PricingCalculator with all billing types**

```rust
use llm_gateway_storage::{PricingPolicy, Usage};

pub struct PricingCalculator;

impl PricingCalculator {
    pub fn calculate_cost(&self, policy: &PricingPolicy, usage: &Usage) -> f64 {
        let cfg = &policy.config;
        match policy.billing_type.as_str() {
            "per_token" => self.calculate_per_token(cfg, usage),
            "per_request" => self.calculate_per_request(cfg, usage),
            "per_character" => self.calculate_per_character(cfg, usage),
            "tiered_token" => self.calculate_tiered_token(cfg, usage),
            "hybrid" => self.calculate_hybrid(cfg, usage),
            _ => 0.0, // Unknown billing type, safe default
        }
    }

    fn calculate_per_token(&self, config: &serde_json::Value, usage: &Usage) -> f64 {
        let input_price = config.get("input_per_1k")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let output_price = config.get("output_per_1k")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        
        let input_cost = (usage.input_tokens as f64 / 1000.0) * input_price;
        let output_cost = (usage.output_tokens as f64 / 1000.0) * output_price;
        input_cost + output_cost
    }

    fn calculate_per_request(&self, config: &serde_json::Value, usage: &Usage) -> f64 {
        let price_per_call = config.get("price_per_call")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        usage.request_count as f64 * price_per_call
    }

    fn calculate_per_character(&self, config: &serde_json::Value, usage: &Usage) -> f64 {
        let input_price = config.get("input_per_1k")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let output_price = config.get("output_per_1k")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        
        let input_chars = usage.input_chars.unwrap_or(0);
        let output_chars = usage.output_chars.unwrap_or(0);
        
        (input_chars as f64 / 1000.0) * input_price + (output_chars as f64 / 1000.0) * output_price
    }

    fn calculate_tiered_token(&self, config: &serde_json::Value, usage: &Usage) -> f64 {
        let tiers = config.get("tiers").and_then(|v| v.as_array());
        if tiers.is_none() {
            return 0.0;
        }
        
        let tiers = tiers.unwrap();
        let mut total_cost = 0.0;
        let mut remaining_input = usage.input_tokens;
        let mut remaining_output = usage.output_tokens;
        
        for tier in tiers {
            let up_to = tier.get("up_to").and_then(|v| v.as_i64());
            let input_rate = tier.get("input_per_1k").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let output_rate = tier.get("output_per_1k").and_then(|v| v.as_f64()).unwrap_or(0.0);
            
            let tier_input = up_to.map(|u| remaining_input.min(u as i64)).unwrap_or(remaining_input);
            let tier_output = up_to.map(|u| remaining_output.min(u as i64)).unwrap_or(remaining_output);
            
            total_cost += (tier_input as f64 / 1000.0) * input_rate + (tier_output as f64 / 1000.0) * output_rate;
            
            if let Some(limit) = up_to {
                remaining_input = remaining_input.saturating_sub(limit as i64);
                remaining_output = remaining_output.saturating_sub(limit as i64);
            } else {
                break;
            }
        }
        
        total_cost
    }

    fn calculate_hybrid(&self, config: &serde_json::Value, usage: &Usage) -> f64 {
        let base = config.get("base_per_call")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let usage_cost = self.calculate_per_token(config, usage);
        (usage.request_count as f64 * base) + usage_cost
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_policy(billing_type: &str, config: serde_json::Value) -> PricingPolicy {
        PricingPolicy {
            id: "test".to_string(),
            name: "Test".to_string(),
            billing_type: billing_type.to_string(),
            config,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_per_token() {
        let calc = PricingCalculator;
        let policy = make_policy("per_token", json!({"input_per_1k": 3.0, "output_per_1k": 15.0}));
        let usage = Usage { input_tokens: 1_000_000, output_tokens: 500_000, input_chars: None, output_chars: None, request_count: 1 };
        let cost = calc.calculate_cost(&policy, &usage);
        assert!((cost - 10.5).abs() < 0.001);
    }

    #[test]
    fn test_per_request() {
        let calc = PricingCalculator;
        let policy = make_policy("per_request", json!({"price_per_call": 0.05}));
        let usage = Usage { input_tokens: 0, output_tokens: 0, input_chars: None, output_chars: None, request_count: 100 };
        let cost = calc.calculate_cost(&policy, &usage);
        assert!((cost - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_tiered_token() {
        let calc = PricingCalculator;
        let policy = make_policy("tiered_token", json!({
            "tiers": [
                {"up_to": 1000000, "input_per_1k": 5.0, "output_per_1k": 15.0},
                {"up_to": null, "input_per_1k": 4.0, "output_per_1k": 12.0}
            ]
        }));
        // 1M tokens at first tier rate
        let usage = Usage { input_tokens: 1_000_000, output_tokens: 0, input_chars: None, output_chars: None, request_count: 1 };
        let cost = calc.calculate_cost(&policy, &usage);
        assert!((cost - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_hybrid() {
        let calc = PricingCalculator;
        let policy = make_policy("hybrid", json!({"base_per_call": 0.001, "input_per_1k": 1.0, "output_per_1k": 3.0}));
        let usage = Usage { input_tokens: 1_000_000, output_tokens: 500_000, input_chars: None, output_chars: None, request_count: 10 };
        let cost = calc.calculate_cost(&policy, &usage);
        // base: 10 * 0.001 = 0.01
        // usage: (1M/1k * 1.0) + (500k/1k * 3.0) = 1000 + 1500 = 2500
        assert!((cost - 2500.01).abs() < 0.001);
    }
}
```

- [ ] **Step 2: Update billing crate lib.rs to export PricingCalculator**

```rust
pub mod pricing;

pub use pricing::PricingCalculator;
pub use llm_gateway_storage::{PricingPolicy, Usage};
```

- [ ] **Step 3: Commit**

```bash
git add crates/billing/src/pricing.rs crates/billing/src/lib.rs
git commit -m "feat: add PricingCalculator with per_token, per_request, tiered_token, hybrid billing types"
```

---

## Task 9: Update API to use PricingCalculator

**Files:**
- Modify: `crates/api/src/openai.rs`, `crates/api/src/anthropic.rs`

- [ ] **Step 1: Find billing calculation in openai.rs (around line 267)**

Replace the current billing logic:

```rust
// OLD:
let billing_type = model_entry.model.billing_type.clone();
// ... calculate using old function

// NEW:
let usage = Usage::from_tokens(
    Some(usage.input_tokens as i64),
    Some(usage.output_tokens as i64),
    1,
);
let calculator = PricingCalculator;
// Get policy from storage or fallback to model
let cost = if let Some(policy_id) = &model_entry.model.pricing_policy_id {
    if let Some(policy) = state.storage.get_pricing_policy(policy_id).await? {
        calculator.calculate_cost(&policy, &usage)
    } else {
        0.0
    }
} else {
    // Fallback to legacy pricing if no policy (backwards compat)
    llm_gateway_billing::calculate_cost(
        &model_entry.model.billing_type.into(),
        Some(usage.input_tokens as i64),
        Some(usage.output_tokens as i64),
        model_entry.model.input_price,
        model_entry.model.output_price,
        model_entry.model.request_price,
    ).cost
};
```

- [ ] **Step 2: Same changes in anthropic.rs**

- [ ] **Step 3: Commit**

```bash
git add crates/api/src/openai.rs crates/api/src/anthropic.rs
git commit -m "refactor: use PricingCalculator in API handlers"
```

---

## Task 10: Add pricing policy API endpoints

**Files:**
- Create: `crates/api/src/management/pricing_policies.rs`
- Modify: `crates/api/src/management/mod.rs`

- [ ] **Step 1: Create pricing_policies.rs handlers**

```rust
use crate::error::ApiResult;
use llm_gateway_storage::{PricingPolicy, CreatePricingPolicy, Storage};
use actix_web::{web, HttpResponse};

pub async fn create(
    state: web::Data<AppState>,
    payload: web::Json<CreatePricingPolicy>,
) -> ApiResult<HttpResponse> {
    let policy = PricingPolicy {
        id: uuid::Uuid::new_v4().to_string(),
        name: payload.name.clone(),
        billing_type: payload.billing_type.clone(),
        config: payload.config.clone(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let created = state.storage.create_pricing_policy(&policy).await?;
    Ok(HttpResponse::Created().json(created))
}

pub async fn list(state: web::Data<AppState>) -> ApiResult<HttpResponse> {
    let policies = state.storage.list_pricing_policies().await?;
    Ok(HttpResponse::Ok().json(policies))
}

pub async fn get(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    let policy = state.storage.get_pricing_policy(&path).await?;
    match policy {
        Some(p) => Ok(HttpResponse::Ok().json(p)),
        None => Ok(HttpResponse::NotFound().json("not found")),
    }
}

pub async fn delete(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> ApiResult<HttpResponse> {
    state.storage.delete_pricing_policy(&path).await?;
    Ok(HttpResponse::NoContent().into())
}
```

- [ ] **Step 2: Register routes in management/mod.rs**

```rust
pub mod pricing_policies;

// In configure() function:
.scope("/pricing-policies")
    .route("", web::post().to(pricing_policies::create))
    .route("", web::get().to(pricing_policies::list))
    .route("/{id}", web::get().to(pricing_policies::get))
    .route("/{id}", web::delete().to(pricing_policies::delete))
```

- [ ] **Step 3: Commit**

```bash
git add crates/api/src/management/pricing_policies.rs crates/api/src/management/mod.rs
git commit -m "feat: add pricing policy API endpoints"
```

---

## Task 11: Run tests and verify

**Files:**
- Test existing tests in `crates/api/tests/`

- [ ] **Step 1: Run cargo test**

```bash
cd /workspace/crates
cargo test --workspace
```

- [ ] **Step 2: Fix any test failures**

- [ ] **Step 3: Commit final changes**

```bash
git add .
git commit -m "test: verify pricing policies implementation"
```

---

## Execution

Plan complete and saved to `docs/superpowers/plans/2026-04-15-pricing-policies.md`. Two execution options:

1. **Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

2. **Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?