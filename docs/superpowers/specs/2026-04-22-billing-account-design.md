# 扣费系统设计

## 1. 概述

为 LLM Gateway 添加用户级余额管理与异步扣费功能。系统记录每次资金变动（充值、消费、退款），支持管理员手动充值，未来可扩展集成真实支付系统（Stripe 等）。

## 2. 数据模型

### 2.1 accounts 表（新增）

```sql
CREATE TABLE accounts (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL UNIQUE REFERENCES users(id),
  balance NUMERIC NOT NULL DEFAULT 0,
  threshold NUMERIC NOT NULL DEFAULT 1.0,
  currency TEXT NOT NULL DEFAULT 'USD',
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_accounts_user_id ON accounts(user_id);
```

- `balance`：当前余额（美元）
- `threshold`：低于此值时拒绝请求（默认 $1.0）
- `currency`：支持未来多币种扩展
- 用户注册时自动创建对应 account

### 2.2 transactions 表（新增）

```sql
CREATE TABLE transactions (
  id TEXT PRIMARY KEY,
  account_id TEXT NOT NULL REFERENCES accounts(id),
  type TEXT NOT NULL,        -- credit | debit | credit_adjustment | debit_refund
  amount NUMERIC NOT NULL,   -- 正数，美元
  balance_after NUMERIC NOT NULL,
  description TEXT,
  reference_id TEXT,          -- 关联 UsageRecord.id 或 payment order id
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_transactions_account_id ON transactions(account_id);
CREATE INDEX idx_transactions_type ON transactions(type);
CREATE INDEX idx_transactions_created_at ON transactions(created_at);
```

- `balance_after`：变动后的余额快照（方便快速查询历史状态）
- `reference_id`：幂等性保障，相同 reference_id 不会重复扣费
- `type` 说明：
  - `credit`：充值
  - `debit`：消费扣费
  - `credit_adjustment`：手动调账（补偿）
  - `debit_refund`：退款

## 3. 结算流程（异步批量）

### 3.1 后台任务

每 N 分钟（可配置，默认 1 分钟）运行结算任务：

```
1. 获取上次结算时间点 T
2. 查询 (T, now] 时间段内的所有 UsageRecords
3. 按 user_id 分组汇总消费金额
4. 对每个用户：
   a. 获取关联的 account
   b. 检查是否已有 reference_id 对应的 debit 记录（幂等）
   c. 余额充足 → 创建 debit 交易，扣除余额
   d. 余额不足 → 记录超支警告，跳过本次扣费（下次继续尝试）
5. 记录结算任务的执行日志
```

### 3.2 并发安全

- 使用分布式锁（PostgreSQL advisory lock 或 SQLite 文件锁）
- 确保同一时间只有一个结算任务运行
- transaction 插入使用乐观锁或 ON CONFLICT 保证幂等

### 3.3 超支处理

- 余额不足以扣费时，不拒绝请求（请求已完成）
- 记录超支状态，管理员可在后台看到超支用户列表
- 下次结算继续尝试扣费

## 4. 请求拦截流程（Pre-check）

在请求认证之后、转发到上游之前，新增余额检查：

```
请求到达
  → 认证通过（API Key → User → Account）
  → 余额预检
       ├─ 余额 ≥ 阈值 → 放行（继续原有流程）
       └─ 余额 < 阈值 → 返回 402 Payment Required
```

- 阈值为 0 时表示不限制
- 充值后自动解除拦截

## 5. 交易保障

### 5.1 幂等性

- `reference_id` 字段关联 UsageRecord.id
- 结算任务先查询是否存在相同 reference_id 的交易，有则跳过

### 5.2 余额一致性

- 每次 debit 操作在事务内完成：查询余额 → 插入 transaction → 更新 account.balance
- 使用数据库行锁保证并发安全

### 5.3 事务边界

- transaction 记录和 account.balance 更新在同一数据库事务内
- 失败回滚，不产生不一致状态

## 6. API 端点

### 6.1 管理员 API

| 方法 | 路径 | 说明 |
|------|------|------|
| `POST` | `/admin/users/{id}/recharge` | 充值 |
| `POST` | `/admin/users/{id}/adjust` | 手动调账（补偿） |
| `POST` | `/admin/users/{id}/refund` | 退款 |
| `GET` | `/admin/users/{id}/balance` | 查询余额 |
| `GET` | `/admin/users/{id}/transactions` | 分页查询交易流水 |
| `PATCH` | `/admin/users/{id}/balance-threshold` | 修改阈值 |

### 6.2 用户 API

| 方法 | 路径 | 说明 |
|------|------|------|
| `GET` | `/me/balance` | 查询自己的余额 |
| `GET` | `/me/transactions` | 查询自己的交易流水 |

### 6.3 错误码

- `402 Payment Required`：余额低于阈值，拒绝请求

## 7. 前端页面

- 用户详情页新增「余额」Tab：
  - 展示当前余额
  - 交易流水列表（分页）
  - 管理员操作：充值、调账按钮
- 用户侧仪表盘展示余额概览

## 8. 数据库迁移

通过 SQL migration 添加：
1. `accounts` 表
2. `transactions` 表
3. `users` 表无需修改（user_id 通过 accounts 表关联）

无破坏性变更，兼容现有数据。

## 9. 扩展点

- **支付集成**：transaction.reference_id 可关联支付订单 ID，webhook 回调后标记支付完成
- **多币种**：accounts.currency 字段预留，UI 和结算逻辑需相应扩展
- **超支告警**：可扩展发送邮件/通知，通知用户余额不足

## 10. 测试策略

- 单元测试：结算金额计算、幂等性、阈值判断
- 集成测试：完整结算流程（需真实数据库）
- 并发测试：多个结算任务同时运行，验证无重复扣费
