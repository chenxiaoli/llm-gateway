# /docs Page Design

## Goal

Add a `/docs` standalone page accessible from the landing page, containing user documentation for both end users and admins.

## Audience

- **End users**: API key management, balance, usage stats
- **Admins**: channel config, provider management, model mapping, pricing policies, rate limits

## Tech Stack

- **MDX**: `@mdx-js/vite` plugin for Vite, `@mdx-js/react` runtime
- **Rendering**: `import.meta.glob` dynamic import to map `(section, slug)` → MDX component
- **Syntax highlighting**: `react-syntax-highlighter`
- **Routing**: React Router nested routes under `/docs`

## File Structure

```
web/
  docs/
    user/
      getting-started.mdx
      api-keys.mdx
      balance.mdx
      usage.mdx
    admin/
      channels.mdx
      providers.mdx
      models.mdx
      pricing-policies.mdx
      rate-limits.mdx
  src/pages/
    DocsLayout.tsx      ← two-column layout: sidebar nav + content
    DocsPage.tsx         ← renders MDX component via import.meta.glob
    DocsNav.tsx          ← navigation sidebar data + rendering
```

## Routing

```tsx
<Route path="/docs" element={<DocsLayout />}>
  <Route index element={<Navigate to="user/getting-started" />} />
  <Route path="user/:slug" element={<DocsPage />} />
  <Route path="admin/:slug" element={<DocsPage />} />
</Route>
```

`DocsLayout` is a two-column layout (sidebar + main content area). `DocsPage` looks up the MDX module by `(section, slug)` and renders it.

## DocsLayout

- Left sidebar (280px): collapsible sections "User Guide" and "Admin Guide", each with links to MDX pages
- Right content: `<Outlet />` renders the active MDX page
- Header: brand logo + language toggle + dark mode toggle + "Dashboard" button (same as Home header)
- Mobile: sidebar collapses to hamburger menu

## DocsNav Data

Driven by static data structure (no dynamic file listing):

```tsx
const docsNav = {
  user: [
    { title: '快速开始', slug: 'getting-started' },
    { title: 'API 密钥管理', slug: 'api-keys' },
    { title: '余额充值', slug: 'balance' },
    { title: '用量统计', slug: 'usage' },
  ],
  admin: [
    { title: '渠道配置', slug: 'channels' },
    { title: '供应商管理', slug: 'providers' },
    { title: '模型管理', slug: 'models' },
    { title: '定价策略', slug: 'pricing-policies' },
    { title: '费率限制', slug: 'rate-limits' },
  ],
};
```

## DocsPage

Uses `import.meta.glob` to lazily load MDX components:

```tsx
const modules = import.meta.glob('../../docs/**/*.mdx')

const DocPage = () => {
  const { section, slug } = useParams()
  const key = `../../docs/${section}/${slug}.mdx`
  const Component = modules[key]()

  return Component ? <Component /> : <NotFound />
}
```

## MDX Content

Each MDX file is a standard Markdown file that can also embed React components. Start with placeholder content that can be expanded over time:

**`web/docs/user/getting-started.mdx`**: Quick start guide
**`web/docs/user/api-keys.mdx`**: API key creation, management
**`web/docs/user/balance.mdx`**: Balance checking, top-up
**`web/docs/user/usage.mdx`**: Usage stats page guide
**`web/docs/admin/channels.mdx`**: Channel configuration guide
**`web/docs/admin/providers.mdx`**: Provider management guide
**`web/docs/admin/models.mdx`**: Model mapping guide
**`web/docs/admin/pricing-policies.mdx`**: Pricing policy guide
**`web/docs/admin/rate-limits.mdx`**: Rate limit configuration guide

## Landing Page Entry

Add "Docs" link to Home.tsx header nav (between language toggle and Dashboard button):

```tsx
<Button variant="ghost" size="sm" onClick={() => navigate('/docs')}>
  {t('home.docs')}
</Button>
```

Add `home.docs` key to i18n (EN: "Docs", ZH: "文档").

## Dependencies

```bash
npm i @mdx-js/vite @mdx-js/react react-markdown remark-gfm react-syntax-highlighter
```

`@mdx-js/vite` goes in `dependencies` (not devDependencies) because it provides runtime components.

## Changes Summary

| File | Change |
|---|---|
| `web/vite.config.ts` | Add `@mdx-js/vite` plugin |
| `web/package.json` | Add mdx/syntax-highlighter dependencies |
| `web/src/App.tsx` | Add `/docs` route under root level |
| `web/src/pages/DocsLayout.tsx` | Create: two-column layout, sidebar nav, header |
| `web/src/pages/DocsPage.tsx` | Create: renders MDX via import.meta.glob |
| `web/src/pages/Home.tsx` | Add "Docs" nav link |
| `web/src/i18n/en.json` | Add `home.docs` key |
| `web/src/i18n/zh.json` | Add `home.docs` key |
| `web/docs/user/*.mdx` | Create 4 user guide MDX files |
| `web/docs/admin/*.mdx` | Create 5 admin guide MDX files |