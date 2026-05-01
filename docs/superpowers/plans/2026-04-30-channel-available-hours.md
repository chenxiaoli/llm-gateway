# Channel Available Hours Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add time-based channel availability so channels can be restricted to specific day-of-week + time ranges for cost optimization.

**Architecture:** Store time slots as JSON in a new `available_hours` TEXT column on `channels`. The routing layer filters channels at resolve time using an `is_available_now()` check against the cached slot data. No new endpoints — existing channel CRUD handles the field.

**Tech Stack:** Rust (Axum, SQLx, serde), SQLite + PostgreSQL migrations, React + TypeScript (Tailwind CSS)

---

## File Structure

| Action | File | Responsibility |
|--------|------|----------------|
| Create | `crates/storage/migrations/sqlite/20260503000000_channel_available_hours.sql` | SQLite migration |
| Create | `crates/storage/migrations/postgres/20260503000000_channel_available_hours.sql` | PostgreSQL migration |
| Modify | `crates/storage/src/types.rs` | TimeSlot struct, Channel/CreateChannel/UpdateChannel fields |
| Modify | `crates/storage/src/sqlite.rs` | Row struct, SELECT/INSERT/UPDATE queries |
| Modify | `crates/storage/src/postgres.rs` | Row struct, SELECT/INSERT/UPDATE queries |
| Modify | `crates/api/src/proxy.rs` | ResolvedChannel field, is_available_now(), resolve filter |
| Modify | `crates/api/src/management/channels.rs` | Request/response DTOs, create/update handlers |
| Modify | `web/src/types/index.ts` | TypeScript TimeSlot and Channel types |
| Modify | `web/src/api/providers.ts` | Update updateChannel to include availableHours |
| Modify | `web/src/pages/ChannelDetail.tsx` | Available Hours card, add/edit/clear UI |

---

### Task 1: Migration

**Files:**
- Create: `crates/storage/migrations/sqlite/20260503000000_channel_available_hours.sql`
- Create: `crates/storage/migrations/postgres/20260503000000_channel_available_hours.sql`

- [ ] **Step 1: Create SQLite migration**

```sql
ALTER TABLE channels ADD COLUMN available_hours TEXT;
```

- [ ] **Step 2: Create PostgreSQL migration**

```sql
ALTER TABLE channels ADD COLUMN available_hours TEXT;
```

- [ ] **Step 3: Build to verify migrations compile**

Run: `cargo build 2>&1 | grep -E "error|Finished"`
Expected: `Finished` with no errors

- [ ] **Step 4: Commit**

```bash
git add crates/storage/migrations/
git commit -m "feat: add available_hours column to channels table"
```

---

### Task 2: Rust Types

**Files:**
- Modify: `crates/storage/src/types.rs`

- [ ] **Step 1: Add TimeSlot struct after the existing imports (after line 8, before the first struct)**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeSlot {
    pub days: Vec<String>,
    pub start: String,
    pub end: String,
}
```

Note: `#[serde(rename_all = "camelCase")]` is NOT needed here because the field names (`days`, `start`, `end`) are already single words. But we include it for consistency and forward compatibility.

- [ ] **Step 2: Add `available_hours` field to `Channel` struct (after `enabled` field, before `created_at`)**

Add at line ~147 (after `pub enabled: bool,`):

```rust
    pub available_hours: Option<Vec<TimeSlot>>,
```

- [ ] **Step 3: Add `available_hours` field to `CreateChannel` struct (after `enabled` field, before `models`)**

Add at line ~164 (after `pub enabled: Option<bool>,`):

```rust
    pub available_hours: Option<Vec<TimeSlot>>,
```

- [ ] **Step 4: Add `available_hours` field to `UpdateChannel` struct (after `weight` field)**

Add at line ~180 (after `pub weight: Option<Option<i32>>,`):

```rust
    pub available_hours: Option<Option<Vec<TimeSlot>>>,
```

Note: Double-Option pattern (`Option<Option<...>>`) matches existing convention in `UpdateChannel` — outer `None` = no change, `Some(None)` = clear to null, `Some(Some(slots))` = set value.

- [ ] **Step 5: Build to verify types compile**

Run: `cargo build 2>&1 | grep -E "error|Finished"`
Expected: Compile errors in sqlite.rs/postgres.rs (missing field) — this is expected, fixed in Task 3.

- [ ] **Step 6: Commit**

```bash
git add crates/storage/src/types.rs
git commit -m "feat: add TimeSlot type and available_hours to Channel structs"
```

---

### Task 3: SQLite Storage Layer

**Files:**
- Modify: `crates/storage/src/sqlite.rs`

- [ ] **Step 1: Add `available_hours` to `SqliteChannelRow` struct (after `weight` field, before `created_at`)**

The struct starts at line 306. Add after the `weight` field:

```rust
    available_hours: Option<String>,
```

- [ ] **Step 2: Update `From<SqliteChannelRow> for Channel` impl to parse the JSON**

In the From impl (line 325), add after `weight: r.weight,`:

```rust
            available_hours: r.available_hours.map(|s| serde_json::from_str(&s).unwrap_or_default()).unwrap_or(None).or_else(|| r.available_hours.map(|_| vec![])),
```

Wait — that's convoluted. Let me be precise. The column stores:
- `NULL` → `None`
- `"[]"` → `Some(vec![])`
- `"[{...}]"` → `Some(vec![TimeSlot{...}])`

Replace the available_hours mapping with:

```rust
            available_hours: match r.available_hours {
                Some(s) if !s.is_empty() => serde_json::from_str(&s).ok(),
                _ => None,
            },
```

Note: `NULL` or empty string → `None` (always available). Valid JSON array → `Some(Vec<TimeSlot>)`.

- [ ] **Step 3: Update all SELECT queries to include `available_hours`**

Every channel SELECT currently has this column list:
```
id, provider_id, name, api_key, base_url, priority, pricing_policy_id, markup_ratio, enabled, rpm_limit, tpm_limit, balance, weight, created_at, updated_at
```

Append `, available_hours` to each SELECT column list. This applies to these methods:
- `get_channel` (line ~825)
- `list_channels` (line ~837)
- `list_channels_by_provider` (line ~848)
- `list_enabled_channels_by_provider` (line ~860)

Each becomes: `... weight, created_at, updated_at, available_hours FROM channels ...`

- [ ] **Step 4: Update `create_channel` INSERT to include `available_hours`**

The INSERT query (line ~749):
```sql
INSERT INTO channels (id, provider_id, name, api_key, base_url, priority, pricing_policy_id, markup_ratio, enabled, rpm_limit, tpm_limit, balance, weight, created_at, updated_at, available_hours)
VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
```

Add one more bind after the last `channel.updated_at.to_rfc3339()`:
```rust
    .bind(channel.available_hours.as_ref().map(|s| serde_json::to_string(s).unwrap()))
```

- [ ] **Step 5: Update `create_channel_with_models` INSERT the same way**

Same change as Step 4 but in the transactional version. Add `available_hours` to the column list, add a `?` placeholder, and add the same bind.

- [ ] **Step 6: Update `update_channel` to include `available_hours`**

The UPDATE query (line ~871):
```sql
UPDATE channels SET name = ?, api_key = ?, base_url = ?, priority = ?, pricing_policy_id = ?, markup_ratio = ?,
 enabled = ?, rpm_limit = ?, tpm_limit = ?, balance = ?, weight = ?, updated_at = ?, available_hours = ? WHERE id = ?
```

Add bind before `.bind(&channel.id)`:
```rust
    .bind(channel.available_hours.as_ref().map(|s| serde_json::to_string(s).unwrap()))
```

- [ ] **Step 7: Build and verify**

Run: `cargo build 2>&1 | grep -E "error|Finished"`
Expected: Errors only in postgres.rs (fixed in Task 4)

- [ ] **Step 8: Commit**

```bash
git add crates/storage/src/sqlite.rs
git commit -m "feat: add available_hours to SQLite channel queries"
```

---

### Task 4: PostgreSQL Storage Layer

**Files:**
- Modify: `crates/storage/src/postgres.rs`

- [ ] **Step 1: Add `available_hours` to `PgChannelRow` struct**

Same as SQLite — add after the `weight` field:

```rust
    available_hours: Option<String>,
```

- [ ] **Step 2: Update `From<PgChannelRow> for Channel` impl**

Add after `weight: r.weight,`:

```rust
            available_hours: match r.available_hours {
                Some(s) if !s.is_empty() => serde_json::from_str(&s).ok(),
                _ => None,
            },
```

- [ ] **Step 3: Update all SELECT queries**

Same as SQLite — append `, available_hours` to the column list in:
- `get_channel`
- `list_channels`
- `list_channels_by_provider`
- `list_enabled_channels_by_provider`

- [ ] **Step 4: Update `create_channel` INSERT**

Add `available_hours` to column list and `$16` placeholder. Add bind:
```rust
    .bind(channel.available_hours.as_ref().map(|s| serde_json::to_string(s).unwrap()))
```

- [ ] **Step 5: Update `create_channel_with_models` INSERT**

Same pattern — add column, placeholder, bind.

- [ ] **Step 6: Update `update_channel`**

Add `available_hours = $13` to SET clause, add bind before `.bind(&channel.id)`:
```rust
    .bind(channel.available_hours.as_ref().map(|s| serde_json::to_string(s).unwrap()))
```

Adjust the `$N` placeholders for `WHERE id = $N` accordingly.

- [ ] **Step 7: Build and verify**

Run: `cargo build 2>&1 | grep -E "error|Finished"`
Expected: `Finished` — all storage layer changes complete

- [ ] **Step 8: Commit**

```bash
git add crates/storage/src/postgres.rs
git commit -m "feat: add available_hours to PostgreSQL channel queries"
```

---

### Task 5: API Layer

**Files:**
- Modify: `crates/api/src/management/channels.rs`

- [ ] **Step 1: Add `available_hours` to `ChannelResponse` struct**

Add field:
```rust
    pub available_hours: Option<Vec<TimeSlot>>,
```

Import `TimeSlot` at the top:
```rust
use llm_gateway_storage::types::TimeSlot;
```

- [ ] **Step 2: Update `ChannelResponse::from(Channel)` impl**

Add after the `weight` mapping:
```rust
            available_hours: channel.available_hours.clone(),
```

- [ ] **Step 3: Add `available_hours` to `ChannelWithModels` struct**

Add field:
```rust
    pub available_hours: Option<Vec<TimeSlot>>,
```

And in its `From<Channel>` impl, add:
```rust
            available_hours: channel.available_hours.clone(),
```

- [ ] **Step 4: Add `available_hours` to `CreateChannelRequest`**

Add field:
```rust
    pub available_hours: Option<Vec<TimeSlot>>,
```

- [ ] **Step 5: Add `available_hours` to `UpdateChannelRequest`**

Add field:
```rust
    pub available_hours: Option<Option<Vec<TimeSlot>>>,
```

- [ ] **Step 6: Update `create_channel` handler — set field when constructing Channel**

In the Channel struct assembly (around line 158-173), add:
```rust
        available_hours: req.available_hours,
```

- [ ] **Step 7: Update `update_channel` handler — apply field from request**

In the field application block (around line 346-377), add:
```rust
    if let Some(available_hours) = req.available_hours {
        channel.available_hours = available_hours;
    }
```

- [ ] **Step 8: Build and verify**

Run: `cargo build 2>&1 | grep -E "error|Finished"`
Expected: `Finished`

- [ ] **Step 9: Commit**

```bash
git add crates/api/src/management/channels.rs
git commit -m "feat: add available_hours to channel API request/response"
```

---

### Task 6: Routing — Time Slot Availability Check

**Files:**
- Modify: `crates/api/src/proxy.rs`

- [ ] **Step 1: Add `available_hours` field to `ResolvedChannel` struct**

After the `proxy_url` field (line ~48):
```rust
    pub available_hours: Option<Vec<TimeSlot>>,
```

Import at top:
```rust
use llm_gateway_storage::types::TimeSlot;
```

- [ ] **Step 2: Set `available_hours` when building ResolvedChannel in `do_reload()`**

In the `do_reload` method where `ResolvedChannel` is constructed (around line 198-210), add:
```rust
            available_hours: channel.available_hours.clone(),
```

- [ ] **Step 3: Write `is_available_now()` function**

Add as a standalone function before `resolve_by_model`:

```rust
fn is_available_now(slots: &Option<Vec<TimeSlot>>) -> bool {
    let slots = match slots {
        Some(s) if !s.is_empty() => s,
        _ => return true,
    };
    let now = chrono::Utc::now();
    let today = now.format("%a").to_string().to_lowercase();
    let now_minutes = now.format("%H").to_string().parse::<i32>().unwrap() * 60
        + now.format("%M").to_string().parse::<i32>().unwrap();

    slots.iter().any(|slot| {
        if !slot.days.contains(&today) {
            return false;
        }
        let start: i32 = slot.start.split(':')
            .take(2)
            .map(|p| p.parse::<i32>().unwrap_or(0))
            .collect::<Vec<_>>()
            .iter()
            .enumerate()
            .map(|(i, v)| if i == 0 { v * 60 } else { *v })
            .sum();
        let end: i32 = slot.end.split(':')
            .take(2)
            .map(|p| p.parse::<i32>().unwrap_or(0))
            .collect::<Vec<_>>()
            .iter()
            .enumerate()
            .map(|(i, v)| if i == 0 { v * 60 } else { *v })
            .sum();
        now_minutes >= start && now_minutes < end
    })
}
```

- [ ] **Step 4: Apply filter in `resolve_by_model()`**

After the channel list is retrieved (around line 260), add a filter:

```rust
        .filter(|ch| is_available_now(&ch.available_hours))
```

Insert this into the chain where `resolve_by_model` returns the resolved channels. The current code gets channel IDs from `model_index` and maps to `ResolvedChannel`. Add the availability filter before returning.

- [ ] **Step 5: Build and verify**

Run: `cargo build 2>&1 | grep -E "error|Finished"`
Expected: `Finished`

- [ ] **Step 6: Commit**

```bash
git add crates/api/src/proxy.rs
git commit -m "feat: filter channels by available_hours in routing"
```

---

### Task 7: Manual Smoke Test

**Files:** None (testing only)

- [ ] **Step 1: Build and run with fresh database**

```bash
rm -f data/gateway.db config.toml
cargo run &
sleep 2
```

- [ ] **Step 2: Create a provider and channel via API**

```bash
curl -s http://localhost:8080/api/v1/admin/providers -H 'Content-Type: application/json' -d '{"name":"Test","endpoints":{"default":"https://api.example.com"}}' | jq .

curl -s http://localhost:8080/api/v1/admin/channels -H 'Content-Type: application/json' -d '{"provider_id":"<id>","name":"Test Channel","api_key":"sk-test","priority":1}' | jq .
```

- [ ] **Step 3: Update channel with available_hours**

```bash
curl -s -X PATCH http://localhost:8080/api/v1/admin/channels/<id> -H 'Content-Type: application/json' -d '{"availableHours":[{"days":["mon","tue","wed","thu","fri"],"start":"09:00","end":"18:00"}]}' | jq .
```

Expected: Response includes `availableHours` with the slot.

- [ ] **Step 4: Verify NULL round-trip (clear schedule)**

```bash
curl -s -X PATCH http://localhost:8080/api/v1/admin/channels/<id> -H 'Content-Type: application/json' -d '{"availableHours":null}' | jq .
```

Expected: Response shows `availableHours: null`.

- [ ] **Step 5: Kill the server**

```bash
kill %1
```

---

### Task 8: Frontend Types and API Client

**Files:**
- Modify: `web/src/types/index.ts`
- Modify: `web/src/api/providers.ts`

- [ ] **Step 1: Add TimeSlot interface to `web/src/types/index.ts`**

Add before the `Channel` interface:

```typescript
export interface TimeSlot {
  days: string[];
  start: string;
  end: string;
}
```

- [ ] **Step 2: Add `available_hours` to Channel interface**

Add field to the `Channel` interface (after `enabled`):

```typescript
  available_hours?: TimeSlot[] | null;
```

- [ ] **Step 3: Add `available_hours` to CreateChannelRequest**

```typescript
  available_hours?: TimeSlot[] | null;
```

- [ ] **Step 4: Add `available_hours` to UpdateChannelRequest**

```typescript
  available_hours?: TimeSlot[] | null;
```

- [ ] **Step 5: Verify `updateChannel` in `web/src/api/providers.ts` passes the field**

The `updateChannel` function should already pass all fields from `UpdateChannelRequest` in the PATCH body. Verify it doesn't strip `available_hours`. If it constructs the body manually, add `available_hours: input.available_hours`.

- [ ] **Step 6: Build frontend**

Run: `cd web && npm run build 2>&1 | tail -5`
Expected: Build succeeds

- [ ] **Step 7: Commit**

```bash
git add web/src/types/index.ts web/src/api/providers.ts
git commit -m "feat: add TimeSlot type and available_hours to frontend types"
```

---

### Task 9: Frontend UI — Channel Detail Available Hours Card

**Files:**
- Modify: `web/src/pages/ChannelDetail.tsx`

- [ ] **Step 1: Add state for available hours**

Add state variable alongside existing state:

```typescript
const [channelAvailableHours, setChannelAvailableHours] = useState<TimeSlot[] | null>(null);
const [showSlotModal, setShowSlotModal] = useState(false);
const [editingSlot, setEditingSlot] = useState<number | null>(null);
const [slotDays, setSlotDays] = useState<string[]>([]);
const [slotStart, setSlotStart] = useState('09:00');
const [slotEnd, setSlotEnd] = useState('18:00');
```

- [ ] **Step 2: Sync state from channel data in useEffect**

In the existing `useEffect` that syncs local state, add:

```typescript
setChannelAvailableHours(channel.available_hours ?? null);
```

- [ ] **Step 3: Add save handler for available hours**

```typescript
const handleSaveAvailableHours = (slots: TimeSlot[] | null) => {
  if (!channel) return;
  updateChannelMutation.mutate(
    { ...channel, available_hours: slots },
    {
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: ['channels', channel.id] });
      },
    }
  );
};
```

- [ ] **Step 4: Add the Available Hours card to the JSX**

Add after the existing Configuration card, before the Channel Models card:

```tsx
{/* Available Hours */}
<div className="bg-zinc-900 rounded-lg p-6">
  <div className="flex items-center justify-between mb-4">
    <h3 className="text-sm font-medium text-zinc-400">Available Hours</h3>
    <div className="flex gap-2">
      {channelAvailableHours && channelAvailableHours.length > 0 && (
        <button
          onClick={() => handleSaveAvailableHours(null)}
          className="text-xs text-zinc-500 hover:text-zinc-300 transition-colors"
        >
          Clear Schedule
        </button>
      )}
      <button
        onClick={() => {
          setEditingSlot(null);
          setSlotDays([]);
          setSlotStart('09:00');
          setSlotEnd('18:00');
          setShowSlotModal(true);
        }}
        className="text-xs text-emerald-400 hover:text-emerald-300 transition-colors"
      >
        + Add Schedule
      </button>
    </div>
  </div>

  {!channelAvailableHours || channelAvailableHours.length === 0 ? (
    <div className="text-sm text-zinc-500">
      <span className="inline-flex items-center px-2 py-0.5 rounded bg-zinc-800 text-zinc-400 text-xs mr-2">Always available</span>
      This channel is available 24/7.
    </div>
  ) : (
    <div className="space-y-2">
      {channelAvailableHours.map((slot, index) => (
        <div key={index} className="flex items-center justify-between py-2 px-3 rounded bg-zinc-800/50">
          <div className="text-sm text-zinc-300">
            {slot.days.map(d => d.charAt(0).toUpperCase() + d.slice(1)).join(', ')}{' '}
            {slot.start}–{slot.end} UTC
          </div>
          <div className="flex gap-2">
            <button
              onClick={() => {
                setEditingSlot(index);
                setSlotDays([...slot.days]);
                setSlotStart(slot.start);
                setSlotEnd(slot.end);
                setShowSlotModal(true);
              }}
              className="text-xs text-zinc-500 hover:text-zinc-300"
            >
              Edit
            </button>
            <button
              onClick={() => {
                const updated = channelAvailableHours.filter((_, i) => i !== index);
                handleSaveAvailableHours(updated.length > 0 ? updated : null);
              }}
              className="text-xs text-red-500 hover:text-red-400"
            >
              Delete
            </button>
          </div>
        </div>
      ))}
    </div>
  )}
</div>
```

- [ ] **Step 5: Add the slot add/edit modal**

Add before the closing fragment of the return, after existing modals:

```tsx
{showSlotModal && (
  <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50" onClick={() => setShowSlotModal(false)}>
    <div className="bg-zinc-900 border border-zinc-800 rounded-lg p-6 w-full max-w-md" onClick={e => e.stopPropagation()}>
      <h3 className="text-lg font-medium text-zinc-100 mb-4">
        {editingSlot !== null ? 'Edit Time Slot' : 'Add Time Slot'}
      </h3>
      <div className="space-y-4">
        {/* Days */}
        <div>
          <label className="block text-sm text-zinc-400 mb-2">Days</label>
          <div className="flex flex-wrap gap-2">
            {['mon', 'tue', 'wed', 'thu', 'fri', 'sat', 'sun'].map(day => (
              <button
                key={day}
                onClick={() => setSlotDays(prev =>
                  prev.includes(day) ? prev.filter(d => d !== day) : [...prev, day]
                )}
                className={`px-3 py-1 rounded text-xs font-medium transition-colors ${
                  slotDays.includes(day)
                    ? 'bg-emerald-500/20 text-emerald-400 border border-emerald-500/30'
                    : 'bg-zinc-800 text-zinc-400 border border-zinc-700 hover:border-zinc-600'
                }`}
              >
                {day.charAt(0).toUpperCase() + day.slice(1)}
              </button>
            ))}
          </div>
        </div>
        {/* Start time */}
        <div>
          <label className="block text-sm text-zinc-400 mb-1">Start Time (UTC)</label>
          <input
            type="time"
            value={slotStart}
            onChange={e => setSlotStart(e.target.value)}
            className="w-full bg-zinc-800 border border-zinc-700 rounded px-3 py-2 text-sm text-zinc-200 focus:outline-none focus:border-zinc-500"
          />
        </div>
        {/* End time */}
        <div>
          <label className="block text-sm text-zinc-400 mb-1">End Time (UTC)</label>
          <input
            type="time"
            value={slotEnd}
            onChange={e => setSlotEnd(e.target.value)}
            className="w-full bg-zinc-800 border border-zinc-700 rounded px-3 py-2 text-sm text-zinc-200 focus:outline-none focus:border-zinc-500"
          />
        </div>
      </div>
      <div className="flex justify-end gap-3 mt-6">
        <button onClick={() => setShowSlotModal(false)} className="px-4 py-2 text-sm text-zinc-400 hover:text-zinc-200">Cancel</button>
        <button
          onClick={() => {
            if (slotDays.length === 0) return;
            const newSlot: TimeSlot = { days: slotDays, start: slotStart, end: slotEnd };
            const current = channelAvailableHours ?? [];
            let updated: TimeSlot[];
            if (editingSlot !== null) {
              updated = current.map((s, i) => i === editingSlot ? newSlot : s);
            } else {
              updated = [...current, newSlot];
            }
            handleSaveAvailableHours(updated);
            setShowSlotModal(false);
          }}
          disabled={slotDays.length === 0}
          className="px-4 py-2 text-sm bg-emerald-500 text-white rounded hover:bg-emerald-400 disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {editingSlot !== null ? 'Update' : 'Add'}
        </button>
      </div>
    </div>
  </div>
)}
```

- [ ] **Step 6: Update `handleSave` to include available_hours in the existing save flow**

In the existing `handleSave` function (around line 75), add `available_hours: channelAvailableHours` to the update payload.

- [ ] **Step 7: Build frontend and verify**

Run: `cd web && npm run build 2>&1 | tail -5`
Expected: Build succeeds

- [ ] **Step 8: Commit**

```bash
git add web/src/pages/ChannelDetail.tsx
git commit -m "feat: add Available Hours card to channel detail page"
```

---

### Task 10: End-to-End Verification

**Files:** None (testing only)

- [ ] **Step 1: Start backend and frontend**

```bash
rm -f data/gateway.db config.toml
cargo run &
cd web && npm run dev &
```

- [ ] **Step 2: Open browser to channel detail page, verify Available Hours card shows "Always available"**

- [ ] **Step 3: Click "Add Schedule", select Mon–Fri 09:00–18:00, save. Verify slot appears.**

- [ ] **Step 4: Edit the slot to add Sat 10:00–14:00. Verify both slots show.**

- [ ] **Step 5: Click "Clear Schedule". Verify it returns to "Always available".**

- [ ] **Step 6: Verify API round-trip via curl**

```bash
curl -s http://localhost:8080/api/v1/admin/channels/<id> | jq '.available_hours'
```

- [ ] **Step 7: Clean up**

```bash
kill %1 %2
```

- [ ] **Step 8: Final commit with all changes**

```bash
git add -A
git commit -m "feat: channel available hours — time-based routing for cost optimization"
```
