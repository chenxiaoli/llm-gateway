# Proxy 混合策略改造设计

## 概述

对 `crates/api/src/proxy.rs` 进行三方增强：SSE 流处理架构、In-Memory Channel Registry、Audit Worker 保持不变。核心目标：减少 DB 查询次数、提升 SSE 处理的可维护性、为测试提供 mock 能力。

## 1. ChannelRegistry Trait

在 `crates/api/src/proxy.rs` 中新增：

```rust
#[derive(Clone, Debug)]
pub struct ResolvedChannel {
    pub channel_id: Uuid,
    pub upstream_base_url: String,
    pub upstream_api_key: String,  // 已解密
    pub adapter: AdapterKind,
    pub timeout_ms: u64,
    pub max_retries: u8,
    pub priority: i32,
    pub pricing_policy_id: Option<String>,
    pub markup_ratio: f64,
    pub upstream_model_name: Option<String>,
    pub provider_id: String,
    pub endpoint_openai: Option<String>,
    pub endpoint_anthropic: Option<String>,
}

#[async_trait::async_trait]
pub trait ChannelRegistry: Send + Sync {
    async fn resolve(&self, channel_id: &str) -> Option<ResolvedChannel>;
    async fn resolve_by_model(&self, model: &str) -> Vec<ResolvedChannel>;
    async fn reload(&self);
}
```

## 2. InMemoryChannelRegistry

实现 `ChannelRegistry` trait，启动时全量加载 + 后台定时刷新：

```rust
pub struct InMemoryChannelRegistry {
    cache: Arc<ArcSwap<HashMap<String, ResolvedChannel>>>,  // key: channel_id
    model_index: Arc<ArcSwap<HashMap<String, Vec<String>>>>, // key: model_name, value: channel_ids
    storage: Arc<dyn Storage>,
    refresh_interval: Duration,
}
```

- **启动加载**：异步加载所有 channels + channel_models + pricing_policies，预解密 API key
- **定时刷新**：后台任务每 N 秒 reload（默认 30s，可配置）
- **reload 逻辑**：清空 HashMap，重新从 Storage 加载，原子替换 ArcSwap
- **proxy 注入**：通过 `AppState.registry: Arc<dyn ChannelRegistry>` 注入

## 3. SSE 流处理改造

采用提案的 mpsc channel 双向架构替代当前的单向累积：

```rust
let (tx, rx) = mpsc::channel::<Result<Event, std::convert::Infallible>>(256);

let pool = state.db_pool.clone();
tokio::spawn(async move {
    process_sse_stream(upstream_resp, tx, pool, request_log_id, start, audit_task_params).await;
});

Sse::new(ReceiverStream::new(rx)).keep_alive(...).into_response()
```

### process_sse_stream 事件级处理

解析 SSE 事件块（包含 `event:`、`id:`、`data:` 行），累积完整事件块字符串用于审计日志：

```
event: message
id: 1
data: {"id":"1","choices":[...]}

event: message
id: 2
data: {"id":"2","choices":[...]}
```

累积时保留完整的 `event:` 和 `id:` 行，存储到 `response_body` 用于审计。

### 与 Audit Worker 的对接

`process_sse_stream` 结束时将 `AuditTask` 发送到 `audit_tx`（保持现有 worker 流程），`task.response_bytes` 填充累积的完整 SSE 事件块文本。

## 4. Proxy 路由改造

### 改造前（当前实现）

每个请求执行多次 DB 查询：

1. `list_models()` — 全量
2. `get_channel_models_for_model()` — 1次
3. `list_channels()` — 全量
4. `get_provider()` — 1次
5. `get_pricing_policy()` — 每候选 channel 1次

### 改造后（缓存命中）

```
resolve_by_model(model) → Vec<ResolvedChannel>  // 内存查找，0 次 DB 查询
```

按 `priority` 排序，遍历候选 channel 做 failover（保持现有行为），API key 已预解密。

缓存 miss 时回退到原 DB 路径（临时行为，等待下次刷新恢复）。

## 5. AppState 变更

```rust
pub struct AppState {
    pub storage: Arc<dyn Storage>,
    pub rate_limiter: Arc<RateLimiter>,
    pub audit_logger: Arc<AuditLogger>,
    pub jwt_secret: String,
    pub encryption_key: [u8; 32],
    pub audit_tx: mpsc::Sender<AuditTask>,
    pub registry: Arc<dyn ChannelRegistry>,  // 新增
}
```

## 6. 配置

在 `config.toml` 或环境变量中新增：

```toml
[registry]
refresh_interval_secs = 30
```

## 7. 测试支持

```rust
struct StubRegistry(ResolvedChannel);
#[async_trait::async_trait]
impl ChannelRegistry for StubRegistry {
    async fn resolve(&self, id: &str) -> Option<ResolvedChannel> {
        Some(self.0.clone())
    }
    async fn resolve_by_model(&self, _: &str) -> Vec<ResolvedChannel> {
        vec![self.0.clone()]
    }
    async fn reload(&self) {}
}
```

## 8. 不变更的部分

- **Audit Worker**：保持现有 `workers.rs` 不变，继续通过 channel 接收 `AuditTask`
- **Error 类型**：保持现有 `ApiError`，不引入 `ProxyError`
- **Protocol 分支**：保持 `ProxyProtocol::OpenAI / Anthropic` 路由逻辑
- **Channel Failover**：保持现有按 priority 遍历的行为

## 9. 实现顺序

1. 定义 `ChannelRegistry` trait + `ResolvedChannel`
2. 实现 `InMemoryChannelRegistry` + 定时刷新
3. 将 registry 注入 `AppState`，在 `lib.rs` 或 `main.rs` 初始化
4. 改造 SSE 流处理：替换为 mpsc channel 架构
5. 改造 proxy 路由：缓存命中走 registry，miss 回退 DB
6. 添加单元测试（StubRegistry）
