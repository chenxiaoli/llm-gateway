# OpenAI /v1/responses 端点实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 添加 `/v1/responses` 端点，完全兼容 OpenAI Responses API，透明代理所有请求。

**Architecture:** 新增 `responses()` 函数，复用现有 `proxy_inner` 逻辑，OpenAI protocol，路径 `/v1/responses`。

**Tech Stack:** Rust, Axum, 现有 proxy 架构

---

## 文件变更概览

| 文件 | 变更 |
|------|------|
| `crates/api/src/proxy.rs` | 新增 `responses()` 函数 |
| `crates/gateway/src/main.rs` | 注册 `/v1/responses` 路由 |

---

### Task 1: 新增 responses() handler

**Files:**
- Modify: `crates/api/src/proxy.rs` (在 `messages()` 函数后新增)

- [ ] **Step 1: 添加 responses() 函数**

在 `crates/api/src/proxy.rs` 第 1316 行后（`messages()` 函数之后，`#[cfg(test)]` 之前）添加：

```rust
/// Wrapper for /v1/responses - uses OpenAI protocol, passthrough all fields
pub async fn responses(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: String,
) -> Result<axum::response::Response, ApiError> {
    proxy_inner(state, headers, body, ProxyProtocol::OpenAI, "/v1/responses".to_string(), 0).await
}
```

- [ ] **Step 2: 编译验证**

Run: `cargo build -p llm-gateway-api`
Expected: SUCCESS

- [ ] **Step 3: Commit**

```bash
git add crates/api/src/proxy.rs
git commit -m "feat: add /v1/responses handler"
```

---

### Task 2: 注册路由

**Files:**
- Modify: `crates/gateway/src/main.rs`

- [ ] **Step 1: 添加路由**

在 `crates/gateway/src/main.rs` 第 107 行（`/v1/messages` 路由）后添加：

```rust
.route("/v1/responses", post(api::proxy::responses))
```

完整路由块：
```rust
.route("/v1/chat/completions", post(api::proxy::proxy_with_protocol))
.route("/v1/models", get(api::models::list_models))
.route("/api/v1/user/models", get(api::user_models::list_user_models))
.route("/v1/messages", post(api::proxy::messages))
.route("/v1/responses", post(api::proxy::responses))
```

- [ ] **Step 2: 编译验证**

Run: `cargo build --release`
Expected: SUCCESS

- [ ] **Step 3: Commit**

```bash
git add crates/gateway/src/main.rs
git commit -m "feat: register /v1/responses route"
```

---

## 验证测试

1. 启动 backend: `cargo run`
2. 发送测试请求:
```bash
curl -X POST http://localhost:8080/v1/responses \
  -H "Authorization: Bearer <API_KEY>" \
  -H "Content-Type: application/json" \
  -d '{"model": "gpt-4o", "input": "Hello"}'
```

预期：请求透明转发到上游，返回响应格式由上游决定。

---

## 依赖

- 无新增依赖
- 复用现有 proxy_inner 逻辑