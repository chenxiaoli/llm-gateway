# Request ID Visibility and Filtering Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show request IDs on the /console/usage and /admin/logs pages with copy buttons, and add a request ID exact-match filter on the logs page.

**Architecture:** Three layers of changes — backend (Rust) adds `request_id` to the usage API response and adds a `request_id` filter to the logs query; frontend types close the gap between backend responses and TypeScript interfaces; two page components gain a new column with a shared `CopyButton` component.

**Tech Stack:** Rust/Axum backend, React 18 + TypeScript + DaisyUI v5 frontend, sonner toasts, lucide-react icons, react-i18next

---

## File Structure

| Action | File | Responsibility |
|--------|------|----------------|
| Create | `web/src/components/ui/CopyButton.tsx` | Reusable copy-with-feedback button |
| Create | `web/src/components/ui/CopyButton.test.tsx` | Unit tests for CopyButton |
| Modify | `crates/api/src/management/usage.rs:16-53` | Add `request_id` to UsageRecordResponse |
| Modify | `crates/storage/src/types.rs:660-675` | Add `request_id` to LogFilter |
| Modify | `crates/storage/src/postgres.rs:1586-1642` | Add request_id filter condition to query_logs_paginated |
| Modify | `web/src/types/index.ts:105-120` | Add `request_id` to UsageRecord |
| Modify | `web/src/types/index.ts:161-182` | Add `request_id` to AuditLogSummary |
| Modify | `web/src/types/index.ts:189-197` | Add `request_id` to LogFilter |
| Modify | `web/src/pages/Usage.tsx:267-293` | Add request_id column to usage table |
| Modify | `web/src/pages/Logs.tsx:29-56` | Add request_id filter state |
| Modify | `web/src/pages/Logs.tsx:120-175` | Add request_id filter input |
| Modify | `web/src/pages/Logs.tsx:189-266` | Add request_id column to logs table |
| Modify | `web/src/api/logs.ts:4-13` | Pass request_id param to API |
| Modify | `web/src/i18n/en.json` | Add i18n keys |
| Modify | `web/src/i18n/zh.json` | Add i18n keys |

---

### Task 1: Create CopyButton component

**Files:**
- Create: `web/src/components/ui/CopyButton.tsx`
- Create: `web/src/components/ui/CopyButton.test.tsx`

- [ ] **Step 1: Write the failing test**

```tsx
// web/src/components/ui/CopyButton.test.tsx
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, act } from '@testing-library/react';
import { CopyButton } from './CopyButton';

describe('CopyButton', () => {
  beforeEach(() => {
    vi.spyOn(navigator.clipboard, 'writeText').mockResolvedValue(undefined);
  });

  it('renders a copy icon button', () => {
    render(<CopyButton value="test-value" />);
    const button = screen.getByRole('button');
    expect(button).toBeInTheDocument();
  });

  it('copies value to clipboard on click', () => {
    render(<CopyButton value="test-value" />);
    fireEvent.click(screen.getByRole('button'));
    expect(navigator.clipboard.writeText).toHaveBeenCalledWith('test-value');
  });

  it('swaps to check icon after click', () => {
    render(<CopyButton value="test-value" />);
    fireEvent.click(screen.getByRole('button'));
    // After click, the svg should have class containing "text-success"
    const svg = screen.getByRole('button').querySelector('svg');
    expect(svg?.className.baseVal).toContain('text-success');
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd web && npx vitest run src/components/ui/CopyButton.test.tsx`
Expected: FAIL — module `./CopyButton` not found

- [ ] **Step 3: Write the component**

```tsx
// web/src/components/ui/CopyButton.tsx
import { useState } from 'react';
import { Check, Copy } from 'lucide-react';
import { toast } from 'sonner';
import { useTranslation } from 'react-i18next';

interface CopyButtonProps {
  value: string;
  className?: string;
}

export function CopyButton({ value, className }: CopyButtonProps) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);

  const handleCopy = (e: React.MouseEvent) => {
    e.stopPropagation();
    navigator.clipboard.writeText(value);
    setCopied(true);
    toast.success(t('toasts.copiedToClipboard'));
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <button
      type="button"
      onClick={handleCopy}
      className={`btn btn-ghost btn-xs px-1 ${className ?? ''}`}
    >
      {copied
        ? <Check className="h-3.5 w-3.5 text-success" />
        : <Copy className="h-3.5 w-3.5 text-base-content/40" />}
    </button>
  );
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd web && npx vitest run src/components/ui/CopyButton.test.tsx`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add web/src/components/ui/CopyButton.tsx web/src/components/ui/CopyButton.test.tsx
git commit -m "feat(ui): add CopyButton component with visual feedback"
```

---

### Task 2: Add i18n keys

**Files:**
- Modify: `web/src/i18n/en.json`
- Modify: `web/src/i18n/zh.json`

- [ ] **Step 1: Add English keys**

In `web/src/i18n/en.json`, add to the `toasts` object (after `"keyCopiedToClipboard"` at line 146):

```json
"copiedToClipboard": "Copied to clipboard"
```

In the `usage.table` object (after `"time"` at line 600), add as the first entry:

```json
"requestId": "Request ID"
```

In the `logs` namespace (after `"allChannels"` at line 628), add:

```json
"requestId": "Request ID",
"requestIdPlaceholder": "Paste request ID"
```

In the `logs.table` object (after `"time"` at line 631), add as the first entry:

```json
"requestId": "Req ID"
```

- [ ] **Step 2: Add Chinese keys**

In `web/src/i18n/zh.json`, add to the `toasts` object (after `"keyCopiedToClipboard"`):

```json
"copiedToClipboard": "已复制到剪贴板"
```

In the `usage.table` object, add as the first entry:

```json
"requestId": "请求 ID"
```

In the `logs` namespace (after `"allChannels"`), add:

```json
"requestId": "请求 ID",
"requestIdPlaceholder": "粘贴请求 ID"
```

In the `logs.table` object, add as the first entry:

```json
"requestId": "请求 ID"
```

- [ ] **Step 3: Verify frontend builds**

Run: `cd web && npx tsc --noEmit`
Expected: No errors

- [ ] **Step 4: Commit**

```bash
git add web/src/i18n/en.json web/src/i18n/zh.json
git commit -m "feat(i18n): add request ID and copy toast keys"
```

---

### Task 3: Backend — Add request_id to UsageRecordResponse

**Files:**
- Modify: `crates/api/src/management/usage.rs:15-53`

- [ ] **Step 1: Add request_id field to UsageRecordResponse struct**

In `crates/api/src/management/usage.rs`, add `request_id` after the `id` field in the struct (line 17):

```rust
pub struct UsageRecordResponse {
    pub id: String,
    pub request_id: String,
    pub key_id: String,
    // ... rest unchanged
}
```

- [ ] **Step 2: Update the From implementation**

In the `From<UsageRecord> for UsageRecordResponse` impl (line 36), add after `id: r.id,`:

```rust
impl From<UsageRecord> for UsageRecordResponse {
    fn from(r: UsageRecord) -> Self {
        UsageRecordResponse {
            id: r.id,
            request_id: r.request_id,
            key_id: r.key_id,
            // ... rest unchanged
        }
    }
}
```

- [ ] **Step 3: Verify backend compiles**

Run: `cd /workspace/llm-gateway && cargo check --workspace`
Expected: Compiles with no errors

- [ ] **Step 4: Commit**

```bash
git add crates/api/src/management/usage.rs
git commit -m "feat(api): include request_id in usage response"
```

---

### Task 4: Backend — Add request_id filter to LogFilter and query

**Files:**
- Modify: `crates/storage/src/types.rs:660-675`
- Modify: `crates/storage/src/postgres.rs:1586-1642`

- [ ] **Step 1: Add request_id field to LogFilter struct**

In `crates/storage/src/types.rs`, add `request_id` as the first field in `LogFilter` (after line 661):

```rust
#[derive(Debug, Deserialize)]
pub struct LogFilter {
    pub request_id: Option<String>,
    pub key_id: Option<String>,
    // ... rest unchanged
}
```

- [ ] **Step 2: Add request_id filter condition in query_logs_paginated**

In `crates/storage/src/postgres.rs`, inside `query_logs_paginated` (after line 1588, before the existing filter conditions), add:

```rust
if let Some(ref request_id) = filter.request_id {
    conditions.push(format!("a.request_id = ${}", bind_vals.len() + 1));
    bind_vals.push(request_id.clone());
}
```

This goes before the `if let Some(ref user_id)` block at line 1590.

- [ ] **Step 3: Verify backend compiles**

Run: `cd /workspace/llm-gateway && cargo check --workspace`
Expected: Compiles with no errors

- [ ] **Step 4: Commit**

```bash
git add crates/storage/src/types.rs crates/storage/src/postgres.rs
git commit -m "feat(storage): add request_id filter to log queries"
```

---

### Task 5: Frontend — Update TypeScript types

**Files:**
- Modify: `web/src/types/index.ts:105-197`

- [ ] **Step 1: Add request_id to UsageRecord**

In `web/src/types/index.ts`, add `request_id` after `id` in the `UsageRecord` interface (line 106):

```typescript
export interface UsageRecord {
  id: string;
  request_id: string;
  key_id: string;
  // ... rest unchanged
}
```

- [ ] **Step 2: Add request_id to AuditLogSummary**

Add `request_id` after `id` in the `AuditLogSummary` interface (line 162):

```typescript
export interface AuditLogSummary {
  id: string;
  request_id: string | null;
  key_id: string;
  // ... rest unchanged
}
```

- [ ] **Step 3: Add request_id to LogFilter**

Add `request_id` as the first field in `LogFilter` (line 189):

```typescript
export interface LogFilter {
  request_id?: string;
  key_id?: string;
  // ... rest unchanged
}
```

- [ ] **Step 4: Verify TypeScript compiles**

Run: `cd web && npx tsc --noEmit`
Expected: No errors

- [ ] **Step 5: Commit**

```bash
git add web/src/types/index.ts
git commit -m "feat(types): add request_id to UsageRecord, AuditLogSummary, LogFilter"
```

---

### Task 6: Frontend — Update Usage.tsx with request_id column

**Files:**
- Modify: `web/src/pages/Usage.tsx`

- [ ] **Step 1: Add CopyButton import**

At the top of `web/src/pages/Usage.tsx`, add after the existing component imports (after line 12):

```tsx
import { CopyButton } from '../components/ui/CopyButton';
```

- [ ] **Step 2: Add request_id column header**

In the table `<thead>` (line 268), add a new `<th>` before the existing Time header:

```tsx
<th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('usage.table.requestId')}</th>
```

- [ ] **Step 3: Add request_id cell to each row**

In the table `<tbody>` row mapping (line 281), add a new `<td>` before the Time cell:

```tsx
<td className="font-mono text-xs text-base-content/55">
  <div className="flex items-center gap-1">
    <span>{item.request_id?.substring(0, 8) ?? '-'}</span>
    {item.request_id && <CopyButton value={item.request_id} />}
  </div>
</td>
```

- [ ] **Step 4: Update colSpan for empty state**

Change the empty row colSpan from 9 to 10 (line 297):

```tsx
<td colSpan={10} className="text-center py-12 text-base-content/40 text-sm">
```

- [ ] **Step 5: Verify TypeScript compiles**

Run: `cd web && npx tsc --noEmit`
Expected: No errors

- [ ] **Step 6: Commit**

```bash
git add web/src/pages/Usage.tsx
git commit -m "feat(usage): show request ID with copy button in usage table"
```

---

### Task 7: Frontend — Update Logs.tsx with request_id filter and column

**Files:**
- Modify: `web/src/pages/Logs.tsx`
- Modify: `web/src/api/logs.ts`

- [ ] **Step 1: Add CopyButton import**

At the top of `web/src/pages/Logs.tsx`, add after the Badge import (line 18):

```tsx
import { CopyButton } from '../components/ui/CopyButton';
```

- [ ] **Step 2: Add request_id filter state**

In the component state declarations (after line 32), add:

```tsx
const [requestIdFilter, setRequestIdFilter] = useState('');
```

- [ ] **Step 3: Update filter object passed to useLogs**

Change the filter object at line 38 to include `request_id`:

```tsx
const { data, isLoading } = useLogs(
  {
    since: since || undefined,
    until: until || undefined,
    key_id: keyFilter || undefined,
    channel_id: channelFilter || undefined,
    request_id: requestIdFilter || undefined,
  },
  page,
  pageSize,
);
```

- [ ] **Step 4: Update clearFilters to include requestIdFilter**

Add `setRequestIdFilter('');` to the `clearFilters` function (after line 53):

```tsx
const clearFilters = () => {
  setSince('');
  setUntil('');
  setKeyFilter('');
  setChannelFilter('');
  setRequestIdFilter('');
  setPage(1);
};
```

- [ ] **Step 5: Update hasFilters and filterCount**

Change line 56 to include `requestIdFilter`:

```tsx
const hasFilters = since || until || keyFilter || channelFilter || requestIdFilter;
const filterCount = [since, until, keyFilter, channelFilter, requestIdFilter].filter(Boolean).length;
```

- [ ] **Step 6: Add request_id filter input in the filter card**

In the filter inputs flex container (after line 174, before the closing `</div>` of the flex-wrap), add:

```tsx
<div>
  <label className="block text-xs font-semibold uppercase tracking-wider text-base-content/50 mb-1.5">
    {t('logs.requestId')}
  </label>
  <input
    type="text"
    value={requestIdFilter}
    onChange={(e) => { setRequestIdFilter(e.target.value); setPage(1); }}
    placeholder={t('logs.requestIdPlaceholder')}
    className="h-10 rounded-lg border border-base-300 bg-base-200/50 px-3 text-sm text-base-content placeholder:text-base-content/25 focus:outline-none focus:border-accent/60 focus:ring-1 focus:ring-accent/20 transition-colors"
  />
</div>
```

- [ ] **Step 7: Add request_id column header**

In the table `<thead>` (before line 191), add:

```tsx
<th className="text-xs font-semibold uppercase tracking-wider text-base-content/45">{t('logs.table.requestId')}</th>
```

- [ ] **Step 8: Add request_id cell to each row**

In the table `<tbody>` row mapping (after the `<tr>` opening tag at line 219, before the Time cell), add:

```tsx
<td className="mono text-xs text-base-content/55" onClick={(e) => e.stopPropagation()}>
  {log.request_id ? (
    <div className="flex items-center gap-1">
      <span>{log.request_id.substring(0, 8)}</span>
      <CopyButton value={log.request_id} />
    </div>
  ) : (
    <span>-</span>
  )}
</td>
```

The `onClick={(e) => e.stopPropagation()}` prevents the copy click from also opening the detail drawer.

- [ ] **Step 9: Update colSpan for empty state**

Change the empty row colSpan from 11 to 12 (line 207):

```tsx
<td colSpan={12} className="text-center py-12 text-base-content/30">
```

- [ ] **Step 10: Pass request_id in the API call**

In `web/src/api/logs.ts`, add after line 10 (after the `until` param check):

```typescript
if (filter.request_id) params.request_id = filter.request_id;
```

- [ ] **Step 11: Verify TypeScript compiles**

Run: `cd web && npx tsc --noEmit`
Expected: No errors

- [ ] **Step 12: Commit**

```bash
git add web/src/pages/Logs.tsx web/src/api/logs.ts
git commit -m "feat(logs): add request ID filter and column with copy button"
```

---

## Self-Review

### Spec coverage
- Request ID on /console/usage with copy button: Task 6
- Request ID on /admin/logs rows (first 8 chars): Task 7
- Copy button on both pages: Tasks 1, 6, 7
- Request ID filter on /admin/logs (exact match): Tasks 4, 7
- Backend request_id in usage response: Task 3
- Backend request_id filter on logs query: Task 4
- Frontend types updated: Task 5
- i18n keys (English + Chinese): Task 2
- Shared CopyButton component: Task 1

### Placeholder scan
No TBDs, TODOs, or vague steps. All code shown inline.

### Type consistency
- `request_id: String` in Rust `UsageRecord` → `request_id: String` in `UsageRecordResponse` → `request_id: string` in TS `UsageRecord`
- `request_id: Option<String>` in Rust `AuditLogSummary` → `request_id: string | null` in TS `AuditLogSummary`
- `request_id: Option<String>` in Rust `LogFilter` → `request_id?: string` in TS `LogFilter`
- `CopyButton` accepts `value: string` prop consistently in both usage pages
