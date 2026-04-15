# Seed Data Design

> **Date:** 2026-04-15
> **Feature:** Seed popular providers and models on initialization

## Goal

Automatically seed the database with popular LLM providers and their flagship models on first run, providing users with a ready-to-use configuration.

## Providers

| Provider | Base URL | OpenAI Compatible | Anthropic Compatible |
|----------|----------|-------------------|----------------------|
| OpenAI | `https://api.openai.com/v1` | Yes | No |
| Anthropic | `https://api.anthropic.com` | No | Yes |
| MiniMax | `https://api.minimax.chat/v1` | Yes | No |
| GLM | `https://open.bigmodel.cn/api/paas/v4` | Yes | No |

## Models

### OpenAI
- gpt-4o (flagship)
- gpt-4o-mini (cheap)
- gpt-4-turbo
- gpt-3.5-turbo

### Anthropic
- claude-sonnet-4-20250514
- claude-4-opus-20250514 (flagship)
- claude-3-5-sonnet
- claude-3-haiku (cheap)

### MiniMax
- abab6.5s-chat
- abab6.5g-chat (flagship)

### GLM
- glm-4 (flagship)
- glm-4-flash (cheap)
- glm-4-plus

## Model Configuration

All seeded models use:
- `billing_type`: "per_token"
- `input_price` / `output_price`: Reference prices (user can modify)
- `enabled`: true

## Implementation

1. **Seed script** - Rust function to insert providers and models
2. **Trigger** - Run on first startup when no providers exist
3. **Idempotent** - Safe to run multiple times (check existing records)