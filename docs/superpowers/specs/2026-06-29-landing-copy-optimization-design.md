# Landing Page Hero Copy Optimization Design

**Date:** 2026-06-29
**Status:** Approved (brainstorming complete)

## Goal

Update the landing page hero copy so the value proposition is clear to a developer scanning the page for 5 seconds: this gateway is a **drop-in replacement** for the OpenAI and Anthropic APIs, not a fan-out to those providers.

The current copy frames the gateway as something that proxies *to* OpenAI and Anthropic. The new copy frames it as something that *accepts* OpenAI and Anthropic SDK calls unchanged — the difference matters because it tells the developer "you don't have to rewrite your app."

## Scope

**In scope**
- Three string values per locale in `home.hero`:
  - `title1`
  - `title2`
  - `description`

**Out of scope**
- Layout, components, structural markup
- Other hero-adjacent copy (Quick Start, How it works, CTA, footer) — the Quick Start already says "Drop in your existing SDK" and reads correctly with the new hero
- Adding new i18n keys
- Translations beyond zh and en

## Files Changed

- `web/src/i18n/en.json` (modify 3 string values)
- `web/src/i18n/zh.json` (modify 3 string values)

No code changes. `web/src/pages/Home.tsx` is untouched — it already reads `home.hero.title1`, `home.hero.title2`, and `home.hero.description`.

## Final Copy

### English (`en.json`)

```json
"title1": "Drop-in compatible",
"title2": "OpenAI and Anthropic SDKs",
"description": "Switch the baseURL on your existing client. The gateway handles keys, rate limits, cost tracking, and multi-provider failover — no application changes required."
```

### Chinese (`zh.json`)

```json
"title1": "即插即用",
"title2": "兼容 OpenAI 和 Anthropic SDK",
"description": "只需切换现有客户端的 baseURL 即可使用。网关统一管理密钥、限速、成本追踪和多提供商故障转移 —— 无需修改应用代码。"
```

### Why this works

- The title leads with the **positioning** (drop-in compatible), the second line names the **two flagship SDKs**, and the description explains the **value** in one sentence. A developer reading for 5 seconds leaves with the message.
- The Quick Start section right below already says "Drop in your existing SDK — just change the `baseURL`", so hero and Quick Start now speak the same language instead of contradicting each other.
- Names only OpenAI and Anthropic (matching the Quick Start tabs exactly). The "every LLM provider" extensibility story is preserved by the implicit "multi-provider failover" in the description — readers who want to know about other providers can read the docs.
- The em-dash `—` (U+2014) is already used in other zh/en copy, so it's a consistent stylistic choice rather than a new convention.

## Testing

This is a copy change. No new automated tests are added — adding a test that asserts `t('home.hero.title1') === 'Drop-in compatible'` would be brittle (any future copy tweak would break it for no functional reason) and would require pulling i18n into the test setup, which it isn't today.

### Manual verification

1. `cd web && source ~/.nvm/nvm.sh && npm run dev` (starts on :5173)
2. Visit `http://localhost:5173/` — confirm the hero shows "Drop-in compatible / OpenAI and Anthropic SDKs" in English
3. Click the `中` toggle in the header — confirm the hero shows "即插即用 / 兼容 OpenAI 和 Anthropic SDK" in Chinese
4. Read the description sentence in both languages — confirm no double-punctuation, no escaped-HTML artifacts. The description is rendered as `dangerouslySetInnerHTML` but neither version contains `<code>` or other tags.
5. `npm test` — confirm existing tests still pass. No test references these i18n keys.

## Risk & Rollback

**Risk: very low.** Only i18n string values change. No API, no layout, no types, no behavior. The change is visible on the landing page only.

**Rollback:** `git revert` the single commit. Re-running `npm run dev` shows the previous copy immediately.

## Commit Strategy

One commit on `develop` (or a `feature/*` branch merged into develop, per project convention):

```
feat(web): reframe hero copy as OpenAI/Anthropic drop-in

The old copy framed the gateway as something that proxies TO OpenAI
and Anthropic. The new copy frames it as a drop-in replacement for
the OpenAI and Anthropic SDKs — telling developers "you don't have
to rewrite your app", which is the actual value proposition.

Hero title and description only. No layout or component changes.
```

## Open Questions

None at design time. If, after deploying, the new copy underperforms in conversion or A/B testing, the fallback copy variants from the brainstorming session (Approaches B and C) are still on file.
