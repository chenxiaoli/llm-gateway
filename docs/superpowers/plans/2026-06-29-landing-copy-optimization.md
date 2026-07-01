# Landing Page Hero Copy Optimization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Update `home.hero.{title1,title2,description}` in both `en.json` and `zh.json` to reframe the value proposition as a drop-in replacement for OpenAI and Anthropic SDKs.

**Architecture:** Pure i18n string-value change in two JSON files. No code, no tests, no layout, no new keys. The existing `Home.tsx` already reads these keys via `t('home.hero.*')`.

**Tech Stack:** React + react-i18next (consumes the JSON files; no code changes needed). Verification via Vite dev server.

---

## File Structure

- **Modify** `web/src/i18n/en.json` — update 3 string values inside the existing `home.hero` object (lines 1149–1155).
- **Modify** `web/src/i18n/zh.json` — update 3 string values inside the existing `home.hero` object (lines 1149–1155).

No other files are touched. `web/src/pages/Home.tsx` is unchanged.

---

## Spec Reference

The exact final copy and the manual verification steps are defined in `docs/superpowers/specs/2026-06-29-landing-copy-optimization-design.md`. This plan implements that spec.

---

### Task 1: Update English hero copy

**Files:**
- Modify: `web/src/i18n/en.json:1149-1155` (the `home.hero` block)

- [ ] **Step 1: Open the file and locate the `home.hero` block**

Run: `grep -n '"hero"' web/src/i18n/en.json`
Expected output: `1149:    "hero": {`

- [ ] **Step 2: Replace the three string values**

In `web/src/i18n/en.json`, replace the entire `home.hero` block (lines 1149–1155) with:

```json
    "hero": {
      "title1": "Drop-in compatible",
      "title2": "OpenAI and Anthropic SDKs",
      "description": "Switch the baseURL on your existing client. The gateway handles keys, rate limits, cost tracking, and multi-provider failover — no application changes required.",
      "getStarted": "Get Started",
      "seeHowItWorks": "See how it works"
    },
```

Note: `getStarted` and `seeHowItWorks` are preserved unchanged from the existing file. The new `title1`, `title2`, and `description` are the three values being changed.

- [ ] **Step 3: Verify the JSON is valid**

Run: `cd web && source ~/.nvm/nvm.sh && node -e "JSON.parse(require('fs').readFileSync('src/i18n/en.json','utf8')); console.log('en.json OK')"`
Expected output: `en.json OK`

If it errors, fix any trailing-comma or quote issues introduced by the edit.

- [ ] **Step 4: Confirm the diff is what you expect**

Run: `git diff web/src/i18n/en.json`
Expected: exactly three string-value changes inside the `home.hero` block (`title1`, `title2`, `description`). No other lines modified.

---

### Task 2: Update Chinese hero copy

**Files:**
- Modify: `web/src/i18n/zh.json:1149-1155` (the `home.hero` block)

- [ ] **Step 1: Open the file and locate the `home.hero` block**

Run: `grep -n '"hero"' web/src/i18n/zh.json`
Expected output: `1149:    "hero": {`

- [ ] **Step 2: Replace the three string values**

In `web/src/i18n/zh.json`, replace the entire `home.hero` block (lines 1149–1155) with:

```json
    "hero": {
      "title1": "即插即用",
      "title2": "兼容 OpenAI 和 Anthropic SDK",
      "description": "只需切换现有客户端的 baseURL 即可使用。网关统一管理密钥、限速、成本追踪和多提供商故障转移 —— 无需修改应用代码。",
      "getStarted": "开始使用",
      "seeHowItWorks": "了解工作原理"
    },
```

Note: `getStarted` and `seeHowItWorks` are preserved unchanged from the existing file. The em-dash is `——` (two U+2014 chars) for parity with the existing zh copy style in the project.

- [ ] **Step 3: Verify the JSON is valid**

Run: `cd web && source ~/.nvm/nvm.sh && node -e "JSON.parse(require('fs').readFileSync('src/i18n/zh.json','utf8')); console.log('zh.json OK')"`
Expected output: `zh.json OK`

- [ ] **Step 4: Confirm the diff is what you expect**

Run: `git diff web/src/i18n/zh.json`
Expected: exactly three string-value changes inside the `home.hero` block (`title1`, `title2`, `description`). No other lines modified.

---

### Task 3: Run the test suite

**Files:** none (read-only verification)

- [ ] **Step 1: Run the web unit tests**

Run: `cd web && source ~/.nvm/nvm.sh && npm test -- --run`
Expected: all existing tests pass. No test references `home.hero.*` keys today, so this is a regression check only.

If a test fails, investigate before proceeding — the failure must be pre-existing, not introduced by this change. (Run the same command on `main` or `HEAD~1` to confirm.)

---

### Task 4: Manually verify in the browser

**Files:** none (manual verification)

- [ ] **Step 1: Start the dev server**

Run: `cd web && source ~/.nvm/nvm.sh && npm run dev`
Expected: Vite prints a local URL (typically `http://localhost:5173/`). Keep this process running in the background or in another terminal — do not kill it until verification is complete.

- [ ] **Step 2: Verify English copy**

Open the URL in a browser. Confirm the hero shows, top to bottom:
- "Drop-in compatible" (title1, default text color)
- "OpenAI and Anthropic SDKs" (title2, in `text-primary`)
- The description sentence: "Switch the baseURL on your existing client. The gateway handles keys, rate limits, cost tracking, and multi-provider failover — no application changes required."

- [ ] **Step 3: Verify Chinese copy**

Click the `中` toggle in the page header (top right). Confirm the hero now shows:
- "即插即用"
- "兼容 OpenAI 和 Anthropic SDK"
- The Chinese description sentence, ending with "—— 无需修改应用代码。"

- [ ] **Step 4: Verify no rendering artifacts**

Read both descriptions carefully. Confirm:
- No literal `<code>` text appears (the description is rendered as `dangerouslySetInnerHTML` but neither version contains HTML tags, so this is a sanity check).
- No double punctuation, no missing spaces around the em-dash.
- The Quick Start section right below still reads "Drop in your existing SDK — just change the `baseURL`" (Chinese: "直接使用现有 SDK — 只需更改 `<code>baseURL</code>`"). The hero and Quick Start should now speak the same "drop-in" language.

- [ ] **Step 5: Stop the dev server**

Kill the `npm run dev` process when verification is complete (Ctrl+C in its terminal, or `kill <pid>`).

---

### Task 5: Commit

**Files:** none (git operation only)

- [ ] **Step 1: Stage the two i18n files**

Run:
```bash
git add web/src/i18n/en.json web/src/i18n/zh.json
```

- [ ] **Step 2: Verify what's staged**

Run: `git status`
Expected output (in the staged-changes section):
```
modified:   web/src/i18n/en.json
modified:   web/src/i18n/zh.json
```
No other files listed.

- [ ] **Step 3: Commit**

Run:
```bash
git commit -m "$(cat <<'EOF'
feat(web): reframe hero copy as OpenAI/Anthropic drop-in

The old copy framed the gateway as something that proxies TO OpenAI
and Anthropic. The new copy frames it as a drop-in replacement for
the OpenAI and Anthropic SDKs — telling developers "you don't have
to rewrite your app", which is the actual value proposition.

Hero title and description only. No layout or component changes.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

Expected: one commit on the current branch. No other side effects.

---

## Self-Review

- **Spec coverage:** "Files Changed" section of the spec calls out exactly the two i18n files. Tasks 1 and 2 cover them. The "Testing" section of the spec calls out manual verification and `npm test` — Tasks 3 and 4 cover both. ✓
- **Placeholder scan:** No "TBD" or "similar to Task N" in this plan. Every step shows its command and expected output. ✓
- **Type/key consistency:** The i18n keys used in all tasks (`home.hero.title1`, `home.hero.title2`, `home.hero.description`) match what `Home.tsx:136-141` reads via `t('home.hero.*')`. ✓
- **YAGNI:** The spec explicitly says no new automated tests. This plan does not add any. ✓
