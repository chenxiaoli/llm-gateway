# Dashboard 每日 Token 用量折线图

## 概述

为 Dashboard 添加每日 token 用量折线图，支持 7 天/30 天切换。Usage 记录新增 `pricing_policy` 和 `weighted_tokens` 字段，加权 Token 在写入时计算。

## 存储变更

### 新增列

`usage` 表新增两列：

| 列 | 类型 | 说明 |
|---|---|---|
| `pricing_policy` | JSONB, nullable | 写入时的 pricing policy 快照 |
| `weighted_tokens` | BIGINT, default 0 | 加权 Token 数 |

### 加权计算规则

基准：input token 权重 = 1。

| Pricing 类型 | 计算方式 |
|---|---|
| per_token | `input + output × (output_price_1m / input_price_1m) + cache_read × (cache_read_price_1m / input_price_1m)` |
| hybrid | 同 per_token |
| tiered_token | 使用 `tiers[0].input_price_1m` 作为基准 |
| context_tiered_token | 使用 `tiers[0].input_price_1m` 作为基准 |
| per_request | `weighted = input + output` |
| per_character | `weighted = input + output` |
| 无 pricing policy | `weighted = input + output + cache_read` |

### 示例

per_token pricing: `input_price_1m=5, output_price_1m=15, cache_read_price_1m=0.5`

```
weighted = 1000 + 500 × (15/5) + 200 × (0.5/5)
         = 1000 + 1500 + 20
         = 2520
```

## 数据流

```
请求完成 → NATS → usage-worker
  → 查 model 的 pricing policy
  → 计算 weighted_tokens
  → 存储 usage record（含 pricing_policy JSON + weighted_tokens）
```

## 后端 API

### 新增端点: GET /usage/daily

查询参数: `since`, `until`, `key_id`, `user_id`, `model_name`

返回: 按天聚合的数据

```json
[
  {
    "date": "2026-05-17",
    "total_input_tokens": 100000,
    "total_output_tokens": 50000,
    "total_cache_read_tokens": 20000,
    "total_weighted_tokens": 252000,
    "total_cost": 1.25,
    "request_count": 150
  }
]
```

### 现有端点变更

所有返回 UsageRecord 的端点新增 `pricing_policy` 和 `weighted_tokens` 字段。

## 前端

### Dashboard 折线图

- 位置：metric cards 下方
- 组件：recharts `LineChart`
- 切换按钮：7 天 / 30 天
- Y 轴：Weighted Tokens
- X 轴：日期
- 样式：暗色主题，与现有设计一致

### Tooltip

悬浮显示：
- 日期
- Weighted Tokens
- Input / Output / Cache Read Tokens
- Cost

## 文件变更

| 文件 | 变更 |
|------|------|
| `crates/storage/migrations/` | 新增 migration: pricing_policy + weighted_tokens 列 |
| `crates/storage/src/types.rs` | UsageRecord 新增字段 |
| `crates/storage/src/postgres.rs` | 新增 daily aggregation 查询 |
| `crates/storage/src/lib.rs` | Storage trait 新增方法 |
| `crates/usage-worker/` | 写入时计算 weighted_tokens |
| `crates/api/src/management/usage.rs` | 新增 daily endpoint + 更新现有返回 |
| `crates/api/src/management/mod.rs` | 注册路由 |
| `web/src/types/index.ts` | 新增 DailyUsageRecord 类型 + UsageRecord 新增字段 |
| `web/src/api/usage.ts` | 新增 queryDailyUsage 函数 |
| `web/src/hooks/useUsage.ts` | 新增 useDailyUsage hook |
| `web/src/pages/Dashboard.tsx` | 添加折线图 |
| `web/src/i18n/en.json` / `zh.json` | 翻译 |
