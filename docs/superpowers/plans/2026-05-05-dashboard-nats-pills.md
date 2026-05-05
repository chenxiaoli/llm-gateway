# Dashboard NATS Status Pills Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add two NATS stream status pills (USAGE and AUDIT) to the admin dashboard's existing status pills row.

**Architecture:** Reuse the existing `useNatsStatus()` hook (from `web/src/hooks/useSettings.ts`) and the existing `StatusPill` component already defined in `Dashboard.tsx`. Add two pills to the existing flex-wrap row, showing pending message count per stream with color coding (green = healthy, amber = backlog).

**Tech Stack:** React, TypeScript, react-i18next, TanStack Query

---

### Task 1: Add i18n keys

**Files:**
- Modify: `web/src/i18n/en.json:217`
- Modify: `web/src/i18n/zh.json:217`

- [ ] **Step 1: Add English i18n keys**

In `web/src/i18n/en.json`, inside the `dashboard.stats` object after the `"recent": "Recent"` entry (line 217), add two new keys:

```json
"recent": "Recent",
"natsUsage": "USAGE Pending",
"natsAudit": "AUDIT Pending"
```

- [ ] **Step 2: Add Chinese i18n keys**

In `web/src/i18n/zh.json`, inside the `dashboard.stats` object after the `"recent"` entry, add two new keys:

```json
"recent": "Recent",
"natsUsage": "USAGE 待消费",
"natsAudit": "AUDIT 待消费"
```

- [ ] **Step 3: Commit**

```bash
git add web/src/i18n/en.json web/src/i18n/zh.json
git commit -m "feat(dashboard): add i18n keys for NATS status pills"
```

---

### Task 2: Add NATS status pills to Dashboard

**Files:**
- Modify: `web/src/pages/Dashboard.tsx`

- [ ] **Step 1: Import useNatsStatus hook**

At the top of `web/src/pages/Dashboard.tsx`, add the import on line 7 (after the existing `useReducedMotion` import):

```tsx
import { useNatsStatus } from '../hooks/useSettings';
```

- [ ] **Step 2: Add useNatsStatus hook call**

Inside the `Dashboard` function, after line 80 (`const { data: myBalance } = useMyBalance(1, 1);`), add:

```tsx
const { data: natsStatus } = useNatsStatus();
```

- [ ] **Step 3: Derive pending values**

After line 92 (after `successRate`), add:

```tsx
const usagePending = natsStatus?.streams?.find(s => s.name === 'LLM_GATEWAY_USAGE')?.pending_messages ?? 0;
const auditPending = natsStatus?.streams?.find(s => s.name === 'LLM_GATEWAY_AUDIT')?.pending_messages ?? 0;
```

- [ ] **Step 4: Add two StatusPill components**

In the Status Pills section (after line 211, before the closing `</motion.div>` on line 212), add two new pills after the existing three:

```tsx
        <StatusPill
          icon={<Activity className="h-4 w-4" />}
          label={t('dashboard.stats.natsUsage')}
          value={String(usagePending)}
          unit={usagePending > 0 ? '⚠' : '✓'}
        />
        <StatusPill
          icon={<Activity className="h-4 w-4" />}
          label={t('dashboard.stats.natsAudit')}
          value={String(auditPending)}
          unit={auditPending > 0 ? '⚠' : '✓'}
        />
```

- [ ] **Step 5: Commit**

```bash
git add web/src/pages/Dashboard.tsx
git commit -m "feat(dashboard): add NATS stream status pills"
```
