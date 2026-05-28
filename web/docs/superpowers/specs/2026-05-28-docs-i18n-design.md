# Docs Internationalization Design

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the /docs pages fully bilingual (Chinese + English) with route-based language switching.

**Architecture:** Route-based i18n where the language is part of the URL path (`/docs/{lang}/{section}/{slug}`). Each doc has separate `.zh.*` and `.en.*` files. Sidebar nav titles use i18n JSON keys. Language toggle redirects the URL in-place.

**Tech Stack:** react-i18next, React Router v6, MDX via @mdx-js/rollup

---

## Route Structure

### Current
- `/docs` → redirect to `/docs/user/getting-started`
- `/docs/:section/:slug` → render doc

### New
- `/docs` → redirect to `/docs/{defaultLang}/user/getting-started` (reads `i18n.language` for default)
- `/docs/:lang/:section/:slug` → render doc
- `/docs/:section/:slug` (old URL without lang) → redirect to `/docs/{defaultLang}/:section/:slug`
- Invalid `lang` value → fallback to `zh`

### URL Examples
- `/docs/zh/user/getting-started` — Chinese getting started
- `/docs/en/admin/channels` — English channels doc
- `/docs/user/getting-started` → redirects to `/docs/zh/user/getting-started` (or `/en/...` based on stored preference)

---

## File Organization

### Current
```
src/docs/user/
  getting-started.tsx
  api-keys.mdx
  balance.mdx
  usage.mdx
src/docs/admin/
  channels.mdx
  providers.mdx
  models.mdx
  pricing-policies.mdx
  rate-limits.mdx
```

### New
```
src/docs/
  user/
    getting-started.zh.tsx
    getting-started.en.tsx
    api-keys.zh.mdx
    api-keys.en.mdx
    balance.zh.mdx
    balance.en.mdx
    usage.zh.mdx
    usage.en.mdx
  admin/
    channels.zh.mdx
    channels.en.mdx
    providers.zh.mdx
    providers.en.mdx
    models.zh.mdx
    models.en.mdx
    pricing-policies.zh.mdx
    pricing-policies.en.mdx
    rate-limits.zh.mdx
    rate-limits.en.mdx
```

Old unsuffixed files are deleted. Each language gets its own file with full content in that language.

---

## Component Lookup

`DocsPage.tsx` uses a two-level map:

```tsx
const components: Record<string, Record<string, React.ComponentType<any>>> = {
  'getting-started': { zh: GettingStartedZh, en: GettingStartedEn },
  'api-keys': { zh: ApiKeysZh, en: ApiKeysEn },
  // ... etc
};

// In the component:
const { lang, slug } = useParams();
const validLang = ['zh', 'en'].includes(lang) ? lang : 'zh';
const Component = components[slug]?.[validLang];
```

---

## Sidebar & Navigation i18n

### Nav Definition

Change from hardcoded titles to i18n keys:

```tsx
const docsNav = {
  user: [
    { titleKey: 'docs.nav.gettingStarted', slug: 'getting-started' },
    { titleKey: 'docs.nav.apiKeys', slug: 'api-keys' },
    { titleKey: 'docs.nav.balance', slug: 'balance' },
    { titleKey: 'docs.nav.usage', slug: 'usage' },
  ],
  admin: [
    { titleKey: 'docs.nav.channels', slug: 'channels' },
    { titleKey: 'docs.nav.providers', slug: 'providers' },
    { titleKey: 'docs.nav.models', slug: 'models' },
    { titleKey: 'docs.nav.pricingPolicies', slug: 'pricing-policies' },
    { titleKey: 'docs.nav.rateLimits', slug: 'rate-limits' },
  ],
};
```

Sidebar links include the current language: `/docs/{lang}/{section}/{slug}`.

### New i18n Keys

Added to both `en.json` and `zh.json` under a `docs` namespace:

```json
// en.json
"docs": {
  "nav.gettingStarted": "Getting Started",
  "nav.apiKeys": "API Keys",
  "nav.balance": "Balance & Top-up",
  "nav.usage": "Usage Statistics",
  "nav.channels": "Channel Configuration",
  "nav.providers": "Provider Management",
  "nav.models": "Model Management",
  "nav.pricingPolicies": "Pricing Policies",
  "nav.rateLimits": "Rate Limits",
  "notFound": "Document not found",
  "goHome": "Go to Home"
}

// zh.json
"docs": {
  "nav.gettingStarted": "快速开始",
  "nav.apiKeys": "API 密钥管理",
  "nav.balance": "余额充值",
  "nav.usage": "用量统计",
  "nav.channels": "渠道配置",
  "nav.providers": "供应商管理",
  "nav.models": "模型管理",
  "nav.pricingPolicies": "定价策略",
  "nav.rateLimits": "费率限制",
  "notFound": "文档未找到",
  "goHome": "返回首页"
}
```

### Language Toggle

In `DocsLayout.tsx`, the language toggle button changes the URL path instead of just switching i18n state:

```tsx
const toggleLanguage = () => {
  const next = i18n.language === 'zh' ? 'en' : 'zh';
  i18n.changeLanguage(next);
  localStorage.setItem('i18n-language', next);
  // Replace current URL lang segment
  const currentPath = location.pathname;
  const newPath = currentPath.replace(/\/docs\/(zh|en)\//, `/docs/${next}/`);
  navigate(newPath, { replace: true });
};
```

### DocsPage 404

Uses i18n keys: `t('docs.notFound')` and `t('docs.goHome')`.

---

## Files to Change

### Modify
1. `src/App.tsx` — route pattern from `:section/:slug` to `:lang/:section/:slug`, add old-URL redirect
2. `src/pages/DocsLayout.tsx` — sidebar links include `lang`, nav titles use i18n keys, language toggle redirects URL
3. `src/pages/DocsPage.tsx` — extract `lang` from params, two-level component lookup, i18n for 404
4. `src/i18n/en.json` — add `docs.*` keys
5. `src/i18n/zh.json` — add `docs.*` keys

### Create
6. `src/docs/user/getting-started.en.tsx` — English version of getting-started (same logic, English text)
7. `src/docs/user/getting-started.zh.tsx` — rename current `getting-started.tsx`
8. 8 `*.en.mdx` files — English translations of all MDX docs
9. 8 `*.zh.mdx` files — rename current MDX docs

### Delete
10. Old unsuffixed files: `getting-started.tsx`, `api-keys.mdx`, `balance.mdx`, `usage.mdx`, `channels.mdx`, `providers.mdx`, `models.mdx`, `pricing-policies.mdx`, `rate-limits.mdx`

### Update
11. `e2e/docs.spec.ts` — test `/docs/zh/...` URL pattern, verify both languages render correctly

---

## Getting-Started English Content

The English `getting-started.en.tsx` mirrors the Chinese version exactly:
- Same `CopyBtn` / `CodeBlock` components
- Same `window.location.origin` for dynamic base URL
- English text: "Getting Started", "Your Base URL", "Replace Configuration", "Code Examples", "Supported Endpoints"
- Same endpoint paths (`/v1/chat/completions`, `/v1/messages`)
