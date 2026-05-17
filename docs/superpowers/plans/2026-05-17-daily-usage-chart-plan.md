# Dashboard 每日 Token 用量折线图 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为 Dashboard 添加每日 token 用量折线图，usage 记录新增 pricing_policy 快照和 weighted_tokens 字段。

**Architecture:** 在 proxy 层计算 weighted_tokens，通过 NATS UsageEvent 传递到 usage-worker 存入数据库。新增 GET /usage/daily 聚合接口。前端使用 recharts LineChart 展示，支持 7/30 天切换。

**Tech Stack:** Rust (Axum, SQLx, NATS), React, TypeScript, recharts

---

## 文件变更概览

| 文件 | 变更 |
|------|------|
| `crates/storage/migrations/postgres/20260517000000_add_pricing_policy_weighted_tokens.sql` | 新增 migration |
| `crates/storage/src/types.rs` | UsageRecord 新增 pricing_policy + weighted_tokens |
| `crates/nats-publisher/src/lib.rs` | UsageEvent 新增 pricing_policy + weighted_tokens |
| `crates/api/src/workers.rs` | 新增 calculate_weighted_tokens() |
| `crates/api/src/proxy.rs` | 调用时计算 weighted_tokens 并传给 NATS |
| `crates/usage-worker/src/main.rs` | 存储 pricing_policy + weighted_tokens |
| `crates/storage/src/postgres.rs` | 更新 record_usage + 新增 query_daily_usage |
| `crates/storage/src/lib.rs` | Storage trait 新增 query_daily_usage |
| `crates/storage/src/types.rs` | 新增 DailyUsageRecord |
| `crates/api/src/management/usage.rs` | 新增 get_daily_usage + 更新现有响应 |
| `crates/api/src/management/mod.rs` | 注册路由 |
| `web/src/types/index.ts` | 新增 DailyUsageRecord + UsageRecord 字段 |
| `web/src/api/usage.ts` | 新增 queryDailyUsage |
| `web/src/hooks/useUsage.ts` | 新增 useDailyUsage |
| `web/src/pages/Dashboard.tsx` | 折线图 |
| `web/src/i18n/en.json` / `zh.json` | 翻译 |

---

### Task 1: Database Migration

**Files:**
- Create: `crates/storage/migrations/postgres/20260517000000_add_pricing_policy_weighted_tokens.sql`

- [ ] **Step 1: 创建 migration 文件**

```sql
-- Add pricing_policy snapshot and weighted_tokens to usage_records
ALTER TABLE usage_records ADD COLUMN IF NOT EXISTS pricing_policy JSONB;
ALTER TABLE usage_records ADD COLUMN IF NOT EXISTS weighted_tokens BIGINT NOT NULL DEFAULT 0;
```

- [ ] **Step 2: 验证 migration 运行**

Run: `cargo build -p llm-gateway-storage`
Expected: SUCCESS

- [ ] **Step 3: Commit**

```bash
git add crates/storage/migrations/postgres/20260517000000_add_pricing_policy_weighted_tokens.sql
git commit -m "feat: migration add pricing_policy and weighted_tokens columns"
```

---

### Task 2: 后端类型更新

**Files:**
- Modify: `crates/storage/src/types.rs`
- Modify: `crates/nats-publisher/src/lib.rs`

- [ ] **Step 1: UsageRecord 新增字段**

在 `crates/storage/src/types.rs` 的 `UsageRecord` struct 中，在 `cost: i64` 后添加：

```rust
pub pricing_policy: Option<serde_json::Value>,
pub weighted_tokens: i64,
```

- [ ] **Step 2: 新增 DailyUsageRecord struct**

在 `crates/storage/src/types.rs` 中新增：

```rust
#[derive(Debug, serde::Serialize)]
pub struct DailyUsageRecord {
    pub date: String,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cache_read_tokens: i64,
    pub total_cache_creation_tokens: i64,
    pub total_weighted_tokens: i64,
    pub total_cost: i64,
    pub request_count: i64,
}
```

- [ ] **Step 3: UsageEvent 新增字段**

在 `crates/nats-publisher/src/lib.rs` 的 `UsageEvent` struct 中，在 `pub cost: i64,` 后添加：

```rust
pub pricing_policy: Option<serde_json::Value>,
pub weighted_tokens: i64,
```

- [ ] **Step 4: 编译验证**

Run: `cargo build`
Expected: 编译失败（后续 task 修复）

- [ ] **Step 5: Commit**

```bash
git add crates/storage/src/types.rs crates/nats-publisher/src/lib.rs
git commit -m "feat: add pricing_policy and weighted_tokens to types"
```

---

### Task 3: weighted_tokens 计算函数

**Files:**
- Modify: `crates/api/src/workers.rs`

- [ ] **Step 1: 在 `calculate_cost` 函数后新增 `calculate_weighted_tokens` 函数**

在 `crates/api/src/workers.rs` 文件末尾（`calculate_cost` 函数之后）添加：

```rust
/// Calculate weighted tokens from pricing policy.
/// Base: input token weight = 1. Other weights are relative to input_price.
pub fn calculate_weighted_tokens(
    pricing_policy_config: &Option<serde_json::Value>,
    pricing_policy_billing_type: &str,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    cache_read_tokens: Option<i64>,
    cache_creation_tokens: Option<i64>,
) -> i64 {
    let input = input_tokens.unwrap_or(0);
    let output = output_tokens.unwrap_or(0);
    let cache_read = cache_read_tokens.unwrap_or(0);
    let cache_creation = cache_creation_tokens.unwrap_or(0);

    let Some(config) = pricing_policy_config else {
        return input + output + cache_read + cache_creation;
    };

    match pricing_policy_billing_type {
        "per_token" | "hybrid" => {
            weighted_from_config(config, input, output, cache_read, cache_creation)
        }
        "tiered_token" => {
            weighted_from_tiered(config, input, output)
        }
        "context_tiered" => {
            weighted_from_context_tiered(config, input, output, cache_read, cache_creation)
        }
        "per_character" => {
            weighted_from_config(config, input, output, 0, 0)
        }
        _ => input + output, // per_request etc.
    }
}

fn price_weight(config: &serde_json::Value, field: &str, base_price: f64) -> f64 {
    let price = config.get(field).and_then(|v| v.as_f64()).unwrap_or(0.0);
    if base_price > 0.0 { price / base_price } else { 0.0 }
}

fn weighted_from_config(
    config: &serde_json::Value,
    input: i64, output: i64, cache_read: i64, cache_creation: i64,
) -> i64 {
    let base = config.get("input_price_1m").and_then(|v| v.as_f64()).unwrap_or(0.0);
    if base <= 0.0 {
        return input + output + cache_read + cache_creation;
    }
    let w_out = price_weight(config, "output_price_1m", base);
    let w_cr = price_weight(config, "cache_read_price_1m", base);
    let w_cc = price_weight(config, "cache_creation_price_1m", base);
    (input as f64
        + output as f64 * w_out
        + cache_read as f64 * w_cr
        + cache_creation as f64 * w_cc
    ).round() as i64
}

fn weighted_from_tiered(config: &serde_json::Value, input: i64, output: i64) -> i64 {
    let tiers = match config.get("tiers").and_then(|t| t.as_array()) {
        Some(t) => t,
        None => return input + output,
    };
    let first = match tiers.first() {
        Some(t) => t,
        None => return input + output,
    };
    let base = first.get("input_price_1m").and_then(|v| v.as_f64()).unwrap_or(0.0);
    if base <= 0.0 {
        return input + output;
    }
    let w_out = price_weight(first, "output_price_1m", base);
    (input as f64 + output as f64 * w_out).round() as i64
}

fn weighted_from_context_tiered(
    config: &serde_json::Value,
    input: i64, output: i64, cache_read: i64, cache_creation: i64,
) -> i64 {
    let tiers = match config.get("tiers").and_then(|t| t.as_array()) {
        Some(t) => t,
        None => return input + output + cache_read + cache_creation,
    };
    let first = match tiers.first() {
        Some(t) => t,
        None => return input + output + cache_read + cache_creation,
    };
    let base = first.get("input_price_1m").and_then(|v| v.as_f64()).unwrap_or(0.0);
    if base <= 0.0 {
        return input + output + cache_read + cache_creation;
    }
    let w_out = price_weight(first, "output_price_1m", base);
    let w_cr = price_weight(first, "cache_read_price_1m", base);
    let w_cc = price_weight(first, "cache_creation_price_1m", base);
    (input as f64
        + output as f64 * w_out
        + cache_read as f64 * w_cr
        + cache_creation as f64 * w_cc
    ).round() as i64
}
```

- [ ] **Step 2: 编译验证**

Run: `cargo build -p llm-gateway-api`
Expected: SUCCESS

- [ ] **Step 3: Commit**

```bash
git add crates/api/src/workers.rs
git commit -m "feat: add calculate_weighted_tokens function"
```

---

### Task 4: Proxy 层传递 pricing_policy 和 weighted_tokens

**Files:**
- Modify: `crates/api/src/proxy.rs`

- [ ] **Step 1: 找到 UsageEvent 构建位置，添加 pricing_policy 和 weighted_tokens**

在 `crates/api/src/proxy.rs` 中搜索 `UsageEvent {` 的位置。在构建 UsageEvent 时，找到 `cost:` 字段后添加：

```rust
pricing_policy: pricing_policy_config.clone(),
weighted_tokens: crate::workers::calculate_weighted_tokens(
    pricing_policy_config,
    pricing_policy_billing_type,
    input_tokens,
    output_tokens,
    cache_read_tokens,
    cache_creation_tokens,
),
```

注意：`pricing_policy_config` 和 `pricing_policy_billing_type` 变量在该作用域内已经可用（它们用于 `calculate_cost` 调用）。`input_tokens`, `output_tokens`, `cache_read_tokens`, `cache_creation_tokens` 也是可用变量。

- [ ] **Step 2: 编译验证**

Run: `cargo build`
Expected: SUCCESS（可能有 usage-worker 编译错误，下一 task 修复）

- [ ] **Step 3: Commit**

```bash
git add crates/api/src/proxy.rs
git commit -m "feat: pass pricing_policy and weighted_tokens in UsageEvent"
```

---

### Task 5: Usage Worker 和 Storage 更新

**Files:**
- Modify: `crates/usage-worker/src/main.rs`
- Modify: `crates/storage/src/postgres.rs`
- Modify: `crates/storage/src/lib.rs`

- [ ] **Step 1: 更新 usage-worker 传递新字段**

在 `crates/usage-worker/src/main.rs` 中，UsageRecord 构建处（`let record = UsageRecord {`），在 `created_at:` 之后添加：

```rust
pricing_policy: event.pricing_policy,
weighted_tokens: event.weighted_tokens,
```

- [ ] **Step 2: 更新 record_usage SQL**

在 `crates/storage/src/postgres.rs` 中找到 `record_usage` 方法的 INSERT SQL。添加 `pricing_policy` 和 `weighted_tokens` 列：

INSERT 列列表追加: `pricing_policy, weighted_tokens`
VALUES 追加: `$15, $16`
参数绑定追加: `.bind(&usage.pricing_policy).bind(usage.weighted_tokens)`

注意更新 `PgUsageRow` struct 如果有的话，添加对应字段。

- [ ] **Step 3: 更新 SELECT 查询返回新字段**

在所有从 `usage_records` 表 SELECT 的查询中，添加 `pricing_policy, weighted_tokens` 列。

更新 `PgUsageRow` struct 添加：
```rust
pub pricing_policy: Option<serde_json::Value>,
pub weighted_tokens: i64,
```

更新所有 `PgUsageRow` 到 `UsageRecord` 的映射代码。

- [ ] **Step 4: 新增 DailyUsageRecord struct 到 types.rs**

（已在 Task 2 完成）

- [ ] **Step 5: Storage trait 新增 query_daily_usage 方法**

在 `crates/storage/src/lib.rs` 的 `Storage` trait 中添加：

```rust
async fn query_daily_usage(&self, filter: &UsageFilter) -> Result<Vec<DailyUsageRecord>, Box<dyn std::error::Error + Send + Sync>>;
```

- [ ] **Step 6: Postgres 实现 query_daily_usage**

在 `crates/storage/src/postgres.rs` 中实现：

```rust
async fn query_daily_usage(&self, filter: &UsageFilter) -> Result<Vec<DailyUsageRecord>, Box<dyn std::error::Error + Send + Sync>> {
    let mut where_clauses = Vec::new();
    let mut param_idx = 1;

    if let Some(ref user_id) = filter.user_id {
        where_clauses.push(format!("user_id = ${}", param_idx));
        param_idx += 1;
    }
    if let Some(ref key_id) = filter.key_id {
        where_clauses.push(format!("key_id = ${}", param_idx));
        param_idx += 1;
    }
    if let Some(ref model_name) = filter.model_name {
        where_clauses.push(format!("model_name = ${}", param_idx));
        param_idx += 1;
    }
    if let Some(since) = filter.since {
        where_clauses.push(format!("created_at >= ${}", param_idx));
        param_idx += 1;
    }
    if let Some(until) = filter.until {
        where_clauses.push(format!("created_at < ${}", param_idx));
        param_idx += 1;
    }

    let where_sql = if where_clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_clauses.join(" AND "))
    };

    let sql = format!(
        "SELECT DATE(created_at) as date, \
         COALESCE(SUM(input_tokens), 0) as total_input_tokens, \
         COALESCE(SUM(output_tokens), 0) as total_output_tokens, \
         COALESCE(SUM(cache_read_tokens), 0) as total_cache_read_tokens, \
         COALESCE(SUM(cache_creation_tokens), 0) as total_cache_creation_tokens, \
         COALESCE(SUM(weighted_tokens), 0) as total_weighted_tokens, \
         COALESCE(SUM(cost), 0) as total_cost, \
         COUNT(*) as request_count \
         FROM usage_records {} \
         GROUP BY DATE(created_at) \
         ORDER BY date",
        where_sql
    );

    let mut query = sqlx::query_as::<_, (String, i64, i64, i64, i64, i64, i64, i64)>(&sql);

    // Re-bind params in same order
    param_idx = 1;
    if let Some(ref user_id) = filter.user_id {
        query = query.bind(user_id);
    }
    if let Some(ref key_id) = filter.key_id {
        query = query.bind(key_id);
    }
    if let Some(ref model_name) = filter.model_name {
        query = query.bind(model_name);
    }
    if let Some(since) = filter.since {
        query = query.bind(since);
    }
    if let Some(until) = filter.until {
        query = query.bind(until);
    }

    let rows = query.fetch_all(&self.pool).await?;
    Ok(rows.into_iter().map(|(date, inp, out, cr, cc, wt, cost, cnt)| DailyUsageRecord {
        date,
        total_input_tokens: inp,
        total_output_tokens: out,
        total_cache_read_tokens: cr,
        total_cache_creation_tokens: cc,
        total_weighted_tokens: wt,
        total_cost: cost,
        request_count: cnt,
    }).collect())
}
```

- [ ] **Step 7: 编译验证**

Run: `cargo build`
Expected: SUCCESS

- [ ] **Step 8: Commit**

```bash
git add crates/usage-worker/src/main.rs crates/storage/src/postgres.rs crates/storage/src/lib.rs
git commit -m "feat: update worker and storage for pricing_policy and weighted_tokens"
```

---

### Task 6: API 端点 - GET /usage/daily + 更新现有响应

**Files:**
- Modify: `crates/api/src/management/usage.rs`
- Modify: `crates/api/src/management/mod.rs`

- [ ] **Step 1: 更新 UsageRecordResponse 添加新字段**

在 `crates/api/src/management/usage.rs` 中找到 `UsageRecordResponse` struct，添加：

```rust
pub pricing_policy: Option<serde_json::Value>,
pub weighted_tokens: i64,
```

更新从 UsageRecord 到 UsageRecordResponse 的映射代码，添加这两个字段的映射。

- [ ] **Step 2: 新增 DailyUsageResponse struct 和 get_daily_usage handler**

```rust
#[derive(serde::Serialize)]
struct DailyUsageResponse {
    date: String,
    total_input_tokens: i64,
    total_output_tokens: i64,
    total_cache_read_tokens: i64,
    total_cache_creation_tokens: i64,
    total_weighted_tokens: i64,
    total_cost: f64,
    request_count: i64,
}

async fn get_daily_usage(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(filter): Query<UsageFilterQuery>,
) -> Result<Json<Vec<DailyUsageResponse>>, ApiError> {
    let user_id = resolve_user_id(&state, &headers, &filter.user_id)?;
    let usage_filter = UsageFilter {
        key_id: filter.key_id,
        user_id,
        model_name: filter.model_name,
        since: filter.since,
        until: filter.until,
    };
    let records = state.storage.query_daily_usage(&usage_filter).await
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(records.into_iter().map(|r| DailyUsageResponse {
        date: r.date,
        total_input_tokens: r.total_input_tokens,
        total_output_tokens: r.total_output_tokens,
        total_cache_read_tokens: r.total_cache_read_tokens,
        total_cache_creation_tokens: r.total_cache_creation_tokens,
        total_weighted_tokens: r.total_weighted_tokens,
        total_cost: units_to_usd(r.total_cost),
        request_count: r.request_count,
    }).collect()))
}
```

注意：`UsageFilterQuery`、`resolve_user_id`、`units_to_usd` 等辅助函数在该文件中已经存在。

- [ ] **Step 3: 注册路由**

在 `crates/api/src/management/mod.rs` 中，在 `/api/v1/usage/channel-summary` 路由后添加：

```rust
.route("/api/v1/usage/daily", get(usage::get_daily_usage))
```

- [ ] **Step 4: 编译验证**

Run: `cargo build`
Expected: SUCCESS

- [ ] **Step 5: Commit**

```bash
git add crates/api/src/management/usage.rs crates/api/src/management/mod.rs
git commit -m "feat: add GET /usage/daily endpoint and update usage response"
```

---

### Task 7: 前端类型和 API

**Files:**
- Modify: `web/src/types/index.ts`
- Modify: `web/src/api/usage.ts`
- Modify: `web/src/hooks/useUsage.ts`

- [ ] **Step 1: 新增 DailyUsageRecord 类型**

在 `web/src/types/index.ts` 中添加：

```typescript
export interface DailyUsageRecord {
  date: string;
  total_input_tokens: number;
  total_output_tokens: number;
  total_cache_read_tokens: number;
  total_cache_creation_tokens: number;
  total_weighted_tokens: number;
  total_cost: number;
  request_count: number;
}
```

同时在 `UsageRecord` interface 中添加：

```typescript
pricing_policy: Record<string, unknown> | null;
weighted_tokens: number;
```

- [ ] **Step 2: 新增 API 函数**

在 `web/src/api/usage.ts` 中添加：

```typescript
export async function queryDailyUsage(filter: UsageFilter): Promise<DailyUsageRecord[]> {
  const params = new URLSearchParams();
  if (filter.key_id) params.set('key_id', filter.key_id);
  if (filter.user_id) params.set('user_id', filter.user_id);
  if (filter.model_name) params.set('model_name', filter.model_name);
  if (filter.since) params.set('since', filter.since);
  if (filter.until) params.set('until', filter.until);
  const query = params.toString() ? `?${params.toString()}` : '';
  const { data } = await apiClient.get<DailyUsageRecord[]>(`/usage/daily${query}`);
  return data;
}
```

- [ ] **Step 3: 新增 React Query hook**

在 `web/src/hooks/useUsage.ts` 中添加：

```typescript
export function useDailyUsage(filter: UsageFilter) {
  return useQuery({
    queryKey: ['daily-usage', filter],
    queryFn: () => queryDailyUsage(filter),
  });
}
```

- [ ] **Step 4: Commit**

```bash
git add web/src/types/index.ts web/src/api/usage.ts web/src/hooks/useUsage.ts
git commit -m "feat: add DailyUsageRecord type, API, and hook"
```

---

### Task 8: Dashboard 折线图

**Files:**
- Modify: `web/src/pages/Dashboard.tsx`
- Modify: `web/src/i18n/en.json`
- Modify: `web/src/i18n/zh.json`

- [ ] **Step 1: 添加 i18n 翻译**

在 `web/src/i18n/en.json` 的 `dashboard` 部分添加：

```json
"chartTitle": "Daily Token Usage",
"last7Days": "7D",
"last30Days": "30D",
"weightedTokens": "Weighted Tokens",
"inputTokens": "Input",
"outputTokens": "Output",
"cacheReadTokens": "Cache Read",
"cacheCreationTokens": "Cache Write",
"cost": "Cost"
```

在 `web/src/i18n/zh.json` 的 `dashboard` 部分添加：

```json
"chartTitle": "每日 Token 用量",
"last7Days": "7天",
"last30Days": "30天",
"weightedTokens": "加权 Token",
"inputTokens": "输入",
"outputTokens": "输出",
"cacheReadTokens": "缓存读取",
"cacheCreationTokens": "缓存写入",
"cost": "费用"
```

- [ ] **Step 2: Dashboard 添加折线图**

在 `web/src/pages/Dashboard.tsx` 中：

1. 添加 imports:
```typescript
import { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer } from 'recharts';
import { useDailyUsage } from '../hooks/useUsage';
```

2. 在 Dashboard 组件中添加时间范围状态和数据获取:
```typescript
const [chartRange, setChartRange] = useState<7 | 30>(7);
const sinceDate = new Date();
sinceDate.setDate(sinceDate.getDate() - chartRange);
sinceDate.setHours(0, 0, 0, 0);
const { data: dailyData } = useDailyUsage({
  since: sinceDate.toISOString(),
});
```

3. 在 metric cards 的 `</div>` 之后、Status Pills 之前，添加折线图:
```tsx
{/* Daily Usage Chart */}
<div className="rounded-xl border border-base-300/40 bg-base-200/30 p-5">
  <div className="flex items-center justify-between mb-4">
    <h3 className="text-sm font-medium text-base-content/70">{t('dashboard.chartTitle')}</h3>
    <div className="flex gap-1">
      <button
        onClick={() => setChartRange(7)}
        className={`px-3 py-1 rounded-md text-xs font-medium transition-colors ${
          chartRange === 7 ? 'bg-base-300/60 text-base-content' : 'text-base-content/40 hover:text-base-content/60'
        }`}
      >
        {t('dashboard.last7Days')}
      </button>
      <button
        onClick={() => setChartRange(30)}
        className={`px-3 py-1 rounded-md text-xs font-medium transition-colors ${
          chartRange === 30 ? 'bg-base-300/60 text-base-content' : 'text-base-content/40 hover:text-base-content/60'
        }`}
      >
        {t('dashboard.last30Days')}
      </button>
    </div>
  </div>
  <ResponsiveContainer width="100%" height={280}>
    <LineChart data={dailyData || []}>
      <CartesianGrid strokeDasharray="3 3" stroke="rgba(255,255,255,0.06)" />
      <XAxis
        dataKey="date"
        stroke="rgba(255,255,255,0.3)"
        tick={{ fontSize: 11 }}
        tickFormatter={(v: string) => v.slice(5)}
      />
      <YAxis
        stroke="rgba(255,255,255,0.3)"
        tick={{ fontSize: 11 }}
        tickFormatter={(v: number) => v >= 1000000 ? `${(v / 1000000).toFixed(1)}M` : v >= 1000 ? `${(v / 1000).toFixed(0)}K` : String(v)}
      />
      <Tooltip
        contentStyle={{ backgroundColor: '#1a1a2e', border: '1px solid rgba(255,255,255,0.1)', borderRadius: 8, fontSize: 12 }}
        formatter={(value: number, name: string) => {
          const labels: Record<string, string> = {
            total_weighted_tokens: t('dashboard.weightedTokens'),
            total_input_tokens: t('dashboard.inputTokens'),
            total_output_tokens: t('dashboard.outputTokens'),
            total_cache_read_tokens: t('dashboard.cacheReadTokens'),
            total_cache_creation_tokens: t('dashboard.cacheCreationTokens'),
          };
          return [value.toLocaleString(), labels[name] || name];
        }}
        labelFormatter={(label: string) => label}
      />
      <Line
        type="monotone"
        dataKey="total_weighted_tokens"
        stroke="#6366f1"
        strokeWidth={2}
        dot={false}
        activeDot={{ r: 4 }}
      />
    </LineChart>
  </ResponsiveContainer>
</div>
```

- [ ] **Step 3: Commit**

```bash
git add web/src/pages/Dashboard.tsx web/src/i18n/en.json web/src/i18n/zh.json
git commit -m "feat: add daily token usage line chart to dashboard"
```

---

## 验证

1. 重启 backend: `cargo run --bin llm-gateway`
2. 重启 workers: `cargo run --bin llm-gateway-usage-worker`
3. 发送测试请求产生 usage 数据
4. 检查 Dashboard 折线图显示
5. 验证 7 天/30 天切换功能
