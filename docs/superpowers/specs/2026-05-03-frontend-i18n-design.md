# Frontend Internationalization — Design Spec

**Date:** 2026-05-03
**Status:** Approved

## Goal

Add internationalization (i18n) to the React frontend, supporting English (base) and Simplified Chinese. Users toggle language manually via sidebar control. Only client-side UI text is translated — backend API error messages remain in English.

## Library

`react-i18next` + `i18next` — industry standard React i18n library. Provides `useTranslation()` hook, JSON translation files, interpolation, pluralization, and fallback handling.

## File Structure

```
web/src/i18n/
├── index.ts    # i18next init config
├── en.json     # English translations (base language)
└── zh.json     # Simplified Chinese translations
```

## Translation Key Organization

Keys are organized by page/component with a `common` namespace for shared strings:

```json
{
  "common": {
    "save": "Save",
    "cancel": "Cancel",
    "delete": "Delete",
    "edit": "Edit",
    "create": "Create",
    "search": "Search...",
    "loading": "Loading...",
    "confirm": "Confirm",
    "back": "Back"
  },
  "sidebar": {
    "dashboard": "Dashboard",
    "keys": "API Keys",
    "models": "Models",
    ...
  },
  "login": {
    "title": "Sign In",
    "username": "Username",
    "password": "Password",
    "submit": "Sign In",
    "errorInvalid": "Invalid username or password"
  },
  "dashboard": {
    "todayRequests": "Today's Requests",
    "monthlyCost": "Monthly Cost",
    "successRate": "Success Rate"
  },
  ...
}
```

Each page has its own top-level key. Shared UI strings live under `common`.

## Language Switching

- Toggle button in the sidebar (globe icon or language label like "EN / 中文")
- Persists choice to `localStorage` key `i18n-language`
- Default follows browser `navigator.language` (falls back to English if not en/zh)
- Instant switch — no page reload, React re-renders with new translations

## Architecture

1. `i18n/index.ts` — initializes i18next with:
   - `fallbackLng: 'en'`
   - `supportedLngs: ['en', 'zh']`
   - `resources` with inline JSON imports (no lazy loading needed for 2 languages)
   - `lng` read from localStorage or detected from browser

2. Every page/component with hardcoded text:
   - Import `useTranslation`
   - Replace inline strings with `t('key')`
   - Keep interpolation for dynamic values: `t('keys.keyCount', { count: 5 })`

3. Layout sidebar:
   - Add language toggle component
   - Calls `i18next.changeLanguage('zh')` / `i18next.changeLanguage('en')`

## Scope

**~350-400 UI strings** across 22 pages and 12 components:
- Page headings, labels, descriptions
- Button text
- Table column headers
- Placeholder text in inputs
- Toast success/error messages (client-side fallbacks only)
- Sidebar navigation labels
- Empty state messages
- Form validation messages

**78 toast messages** — the hardcoded fallback strings in `toast.error()` / `toast.success()` calls get translation keys. API-returned error messages (from `error.response.data.message`) stay as-is in English.

## Not Changed

- Backend Rust code
- API client logic or error handling
- Component structure or layout
- Route paths
- API request/response format
- Toast library (still `sonner`)

## Estimated String Count by Area

| Area | Approximate Strings |
|---|---|
| Layout sidebar + header | 14 |
| Common (buttons, labels, validation) | 30 |
| Auth (login, register, change password) | 25 |
| Dashboard | 20 |
| Keys + KeyDetail | 30 |
| Models + ConsoleModels | 25 |
| Providers + ProviderDetail | 25 |
| Channels + ChannelDetail | 30 |
| Usage | 15 |
| Logs | 15 |
| Users | 15 |
| Settings | 20 |
| Account + AccountBalance | 15 |
| Model Fallbacks | 15 |
| Pricing Policies | 15 |
| Hooks (toast fallbacks) | 40 |
| UI components (Modal, ConfirmDialog, etc.) | 10 |
| Home page | 20 |
| **Total** | **~380** |
