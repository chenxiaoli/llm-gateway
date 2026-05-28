# 快速开始

欢迎使用 LLM Gateway！本指南将帮助你快速上手。

## 创建 API 密钥

1. 登录控制台，进入 **API 密钥** 页面
2. 点击「创建密钥」按钮
3. 设置密钥名称，点击确认
4. 复制生成的 API 密钥（创建后无法再次查看）

## 基本使用

获取 API 密钥后，你可以通过 OpenAI 兼容的 SDK 接入：

```js
import OpenAI from 'openai';

const client = new OpenAI({
  apiKey: 'sk-your-api-key',
  baseURL: 'https://your-gateway-url/v1',
});

const chat = await client.chat.completions.create({
  model: 'gpt-4o',
  messages: [{ role: 'user', content: 'Hello!' }],
});
```

## Anthropic 协议

也支持 Anthropic 协议：

```js
import Anthropic from '@anthropic-ai/sdk';

const client = new Anthropic({
  apiKey: 'sk-your-api-key',
  baseURL: 'https://your-gateway-url/v1',
});

const message = await client.messages.create({
  model: 'claude-3-5-sonnet-20241022',
  max_tokens: 100,
  messages: [{ role: 'user', content: 'Hello!' }],
});
```