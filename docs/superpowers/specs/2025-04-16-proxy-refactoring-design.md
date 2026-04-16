# Proxy Refactoring Design

## Date: 2025-04-16

## Goal

重构 API 层，实现：
1. 合并 openai.rs + anthropic.rs → proxy.rs（纯转发）
2. 异步处理：usage 统计、audit log、cost 计算

## Current Problems

### 1. Streaming Audit Log Bug（优先修复）
- Stream 请求：audit log 没有保存
- Non-stream 请求：正常

### 2. 代码耦合问题
- `chat_completions` 和 `messages` 职责过多：
  - 认证 + Rate limit
  - 请求路由/failover
  - 用量统计 + cost 计算
  - Audit log（同步）
  - Streaming 处理

### 3. 重复代码
- openai.rs: 661 行
- anthropic.rs: 666 行
- 约 80% 相同

## Proposed Architecture

### Layer 1: Proxy Endpoint (`proxy.rs`)

只做：
1. 解析 Bearer token → 查找 API key
2. 解析 model 名称
3. 查找 provider/channel（用于路由）
4. **转发请求到 upstream**
5. **返回响应**（stream 或 non-stream）

不做：
- 用量计算
- Cost 计算
- Audit log

### Layer 2: Async Workers

独立组件处理（`tokio::spawn` 或消息队列）：

**Usage Worker:**
- 计算 input_tokens/output_tokens
- 写入 usage_records 表
- 计算 cost

**Audit Worker:**
- 记录请求/响应 body
- 记录 latency_ms
- 记录 status_code

### Endpoint Design

```rust
// 统一端点：根据 path 判断协议
POST /v1/chat/completions → OpenAI 协议转发
POST /v1/messages → Anthropic 协议转发
POST /v1/proxy?protocol=openai → 显式指定

// 或者：一个端点自动判断
POST /v1/proxy → 根据 model 配置决定协议
```

## Implementation Phases

### Phase 1: 修复 Streaming Audit Bug（紧急）
- 定位 stream 完成时没有调用 audit log 的代码路径
- 补全缺失的 audit 调用

### Phase 2: 创建 Proxy Layer
- 提取公共请求解析逻辑
- 创建 `proxy_request()` 函数
- 移除同步 usage/audit/cost 代码

### Phase 3: 创建 Async Workers
- 创建 `usage_worker` 模块
- 创建 `audit_worker` 模块
- 修改 proxy 调用 async workers

### Phase 4: 端点整合（可选）
- 合并 `/v1/chat/completions` 和 `/v1/messages`
- 或保持两个端点共享底层 proxy

## Key Questions

1. **Streaming Bug 优先级**：先修复还是先重构？
2. **异步队列**：用 channel + tokio::spawn 还是消息队列（tokio::sync）？
3. **Endpoint**：合并成一个还是保持两个？