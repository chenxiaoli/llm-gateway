# OpenAI /v1/responses 端点支持

## 概述

为 LLM Gateway 添加 `/v1/responses` 端点，完全兼容 OpenAI Responses API 标准。作为透明代理运行：客户端发什么透传给上游，上游返什么原样转发给下游。

## 架构

```
客户端 → /v1/responses → responses() handler
                            ↓
              提取 model 字段用于路由选通道
                            ↓
              proxy_inner(protocol=OpenAI, path="/v1/responses")
                            ↓
              复用现有通道选择 + 模型路由 + 计费逻辑
                            ↓
              上游 provider（原始请求体完整透传）
                            ↓
              原始响应完整转发给客户端
```

## 实现细节

### 1. Handler 函数

`crates/api/src/proxy.rs` 新增：

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

### 2. 路由注册

`crates/gateway/src/main.rs`：

```rust
.route("/v1/responses", post(api::proxy::responses))
// 详情查询
.route("/v1/responses/{response_id}", get(api::proxy::responses_detail))
```

### 3. 请求处理

`proxy_inner` 逻辑复用 OpenAI adapter：

1. **认证**：从 Authorization header 提取 Bearer token 验证 API key
2. **余额检查**：检查用户账户余额是否充足
3. **模型提取**：从 `model` 字段提取模型名用于路由
4. **通道选择**：按 priority + weight 选择可用通道
5. **上游调用**：将原始请求体完整透传给上游
6. **响应转发**：原始响应完整转发给客户端

### 4. 透明代理原则

- 输入：不做解析、不做转换、所有字段原样保留
- 输出：不做转换、原样转发
- 支持字段（包括但不限于）：
  - `model` - 路由用
  - `input` - string 或 array
  - `modalities` - 输出格式
  - `previous_response_id` - 多轮对话上下文
  - `temperature`、`top_p`、`max_tokens` 等生成参数
  - 流式参数 `stream`
- 支持 input 内容类型：
  - 纯文本 text
  - 图片 image_url
  - 音频 audio
  - 多模态混合

## 文件变更

| 文件 | 变更 |
|------|------|
| `crates/api/src/proxy.rs` | 新增 `responses()` 和 `responses_detail()` 函数 |
| `crates/gateway/src/main.rs` | 注册 `/v1/responses` 路由 |

## 测试计划

1. **非流式请求**：验证 input 透传和响应返回
2. **流式请求**：验证 SSE 响应转发
3. **多模态输入**：验证 image_url 等多模态内容透传
4. **previous_response_id**：验证多轮对话上下文
5. **通道选择**：验证 priority + weight 路由逻辑

## 暂不实现

- 无

## 依赖

- 复用现有 `proxy_inner` 逻辑，无需新增依赖