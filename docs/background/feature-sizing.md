# Switchboard — Feature Inventory & T-Shirt Sizing

**Date:** 2026-05-27
**Basis:** v0.0.30 — README, `preload.js` API surface, IPC handlers.
**Effort calibration:** Appendix A (git-history extrapolation, adjusted).
**IDs:** roadmap-style `F-<kebab-slug>` with HTML anchors.

## How to read the sizes

**PD = person-day** — one focused engineer-day (~6 productive hours). Not a
calendar day. All numeric estimates below are in PD.

Each feature has **one t-shirt size** = its **build complexity in a restart**:
the effort to reimplement it against a *known target* (the existing app is the
spec, and the MIT domain modules can be lifted). This is deliberately **not** a
"cold rebuild from zero" number — that fiction never applies to a real restart,
where you can read the reference and copy the hard parts. The sizes are anchored
to what the feature actually cost to build (Appendix A, adjusted active-days).

| Size | Rough effort | PD  | Typical driver                        |
| ---- | ------------ | --- | ------------------------------------- |
| S    | ≤ 2 days     | 1.5 | small surface / lift-and-wire         |
| M    | 3–4 days     | 3.5 | real logic, some domain knowledge     |
| L    | ~1 week+     | 6.5 | heavy UI or sustained, broad feature  |

**Distribution.** The shape is **3 L / 7 M / 7 S** (≈ 18 / 41 / 41) — S/M-heavy,
and honestly so: the genuinely large work is concentrated in three features
(the diff UI, the session browser, the terminal), while most everything else is
small once the reference exists and the reverse-engineered modules are lifted.
That is *why* a restart is tractable.

**Reuse (MIT):** several features rest on small, clean, MIT-licensed domain
modules that can be lifted verbatim into a restart with attribution. Where that
shrinks the build, it is noted — these are the features whose cost mostly
*evaporates* in a restart.

---

## Core session features

### `F-session-browser` — Session Browser / sidebar (L) <a id="f-session-browser"></a>

Discover all sessions under `~/.claude/projects`, group by real project path,
list with metadata. The hard part is **path derivation** (`derive-project-path`,
`encode-project-path`) plus the SQLite cache and worker-thread scan. The most
*continuously* touched feature in the history (10 active days).

- Reuse: `derive-project-path.js`, `encode-project-path.js`,
  `read-session-file.js` — lifting these removes most of the hard part.

### `F-builtin-terminal` — Built-in terminal (PTY) (L) <a id="f-builtin-terminal"></a>

`node-pty` + xterm, spawn with a cleaned env, new / resume / reattach flows.
Cross-platform shell handling (zsh/bash/fish/PowerShell/CMD/WSL) and WSL path
translation is the fiddly part.

- Reuse: `shell-profiles.js` — lifting it removes the cross-platform fiddliness.

### `F-fork-detection` — Fork & resume detection (M) <a id="f-fork-detection"></a>

Detect fork / plan-accept by reading new-session JSONL heads and old-session
tails, matched within a 30s window, then re-key the MCP server. Subtle and easy
to get wrong, but small (~200 lines).

- Reuse: `session-transitions.js` (signal extraction) — lifting it drops this
  to a wiring job.

### `F-fts-search` — Full-text search (M) <a id="f-fts-search"></a>

SQLite FTS5 index over session content, case-insensitive, with a title-only
toggle. Index maintenance on every scan.

### `F-session-metadata` — Session metadata (S) <a id="f-session-metadata"></a>

Star / rename / archive, plus title precedence (user rename > AI title > raw
prompt). The exact logic behind the v0.0.30 → #46 regression — tiny code
(~60 churn), but a subtle contract: small size, **high test-coverage priority**.

- Reuse: title precedence lives in `read-session-file.js`.

### `F-jsonl-viewer` — JSONL transcript viewer (M) <a id="f-jsonl-viewer"></a>

Render a session transcript readably (messages, tool calls, inline results).
Mostly UI; pairs with a JSONL parser.

---

## IDE emulation (the moat)

### `F-mcp-ide-bridge` — MCP bridge / IDE server (S) <a id="f-mcp-ide-bridge"></a>

Per-PTY WebSocket MCP server, lock files in `~/.claude/ide`, JSON-RPC 2.0,
`openDiff` / `openFile` / `closeAllDiffTabs` / `getDiagnostics`, per-session
lifecycle and re-keying on fork. The reverse-engineered heart of the product —
but it was **built in a single day** (commit `8905896`, ~490 lines).

- Reuse: `mcp-bridge.js` — this is the prime lift candidate. Its value is the
  *protocol knowledge*, which the module already encodes; in a restart you lift
  it and wire transport/lifecycle. **Sized S as a restart, not for its
  importance** — its importance is exactly why you don't rebuild it from zero.

### `F-file-diff-panel` — File panel / diff review (L) <a id="f-file-diff-panel"></a>

CodeMirror merge view, inline + side-by-side, **partial chunk accept/reject**,
syntax highlighting, OSC 8 clickable file links. UI-heavy and tightly bound to
CodeMirror.

- Reuse: none liftable — pure UI. **The single largest real build** (~2.5k
  churn, 6 active days) and the dominant cost of any restart.

### `F-status-notifications` — Status notifications (M) <a id="f-status-notifications"></a>

Parse PTY output (OSC escape sequences, alternate-screen tracking) to derive
busy / waiting-for-input / permission-blocked states and surface sidebar
badges. Reverse-engineered terminal parsing — brittle, high-value.

- Reuse: none clean (logic is embedded in `main.js`), but the existing code is
  a complete reference to port — no re-derivation needed.

---

## Productivity surfaces

### `F-plans-memory` — Plans & memory browser/editor (S) <a id="f-plans-memory"></a>

Scan plan files and `CLAUDE.md` memories across projects; view and save edits.
Leans on the editor built for the diff panel.

### `F-activity-stats` — Activity stats / heatmap (M) <a id="f-activity-stats"></a>

Usage heatmap across projects. Includes reading Claude OAuth credentials
(macOS Keychain → `.credentials.json` fallback) — the fiddly bit.

- Reuse: `claude-auth.js`.

### `F-session-grid` — Session grid overview (S) <a id="f-session-grid"></a>

Render multiple live xterm terminals in cards, status dots, click-to-focus /
double-click-to-expand, persisted preference. Reuses the terminal manager.

### `F-scheduled-tasks` — Scheduled tasks (M) <a id="f-scheduled-tasks"></a>

`schedule-*.md` frontmatter + cron parsing, a runner, and create/run-now flows.

- Reuse: `schedule-runner.js` (frontmatter + cron) — lifting it drops this to a
  wiring job.

### `F-settings` — Settings (global + per-project effective) (S) <a id="f-settings"></a>

Key/value store, shell-profile selection, IDE-emulation toggle, defaults merge,
effective-settings resolution per project. The merge logic is the only real
part; the rest is forms over a k/v store.

---

## Platform / shell (cross-cutting, not user-facing)

### `F-app-shell` — App shell & window management (S) <a id="f-app-shell"></a>

Window lifecycle, bounds persistence, menu, keyboard shortcuts (`Cmd+F`,
`Cmd+G`), external-link handling.

### `F-auto-update` — Auto-update (S) <a id="f-auto-update"></a>

`electron-updater` against GitHub Releases, background download, restart toast.
Mostly off-the-shelf.

### `F-packaging-ci` — Packaging / signing / CI (M) <a id="f-packaging-ci"></a>

electron-builder multi-platform, macOS notarization + hardened runtime,
Windows NSIS, Linux AppImage/deb, GitHub Actions matrix. Low churn, high
friction (the #18 per-arch fights — wall-clock outran the line count). See the
NFR doc.

---

## Foundation (restart-only, before any feature)

A restart pays these once, up front, regardless of feature set — but so did the
original (inside `db.js`/`main.js`), so they are *included* in the original
~55 PD, not extra:

- `F-foundation-scaffold` — scaffold + chosen stack (renderer framework?
  bundler? TS?) — **S–M**
- `F-foundation-ipc` — IPC architecture / contextBridge surface — **S**
- `F-foundation-db` — SQLite cache + migration layer (clean, not a
  singleton) — **M**
- `F-foundation-tests` — test harness + CI test/lint gate (absent today) — **S**

---

## Roll-up

| Size | Count | %   |
| ---- | ----- | --- |
| L    | 3     | 18% |
| M    | 7     | 41% |
| S    | 7     | 41% |

- **L:** session-browser, builtin-terminal, file-diff-panel
- **M:** fts, jsonl-viewer, activity-stats, scheduled, packaging, fork, status
- **S:** session-metadata, mcp-ide-bridge, plans-memory, grid, settings,
  app-shell, auto-update

The three L's are the real cost of a restart; almost everything else is S/M, and
several of those (`F-mcp-ide-bridge`, `F-fork-detection`, `F-scheduled-tasks`,
`F-session-browser`'s core) shrink further by lifting their MIT modules.

---

## The actual decision: Continue-and-remediate vs Restart

> **This section replaces the earlier A/B/C estimate, which was wrong.** A/B/C
> priced three ways to *build features* and even had a "cold rebuild" costing
> **more** than the original — impossible, since a restart reimplements a known
> target and skips all the original discovery. The real choice is between two
> *different activities*, and the deciding axis is **quality and hazard**, not
> raw build cost.

**Anchor:** the original build cost ≈ **55 PD** (Appendix A) — and that price
*included* all the discovery, dead-ends, and reverse-engineering. Any restart is
bounded above by it and runs lower.

- **Restart, raw (retype to parity):** ~50 PD — sum of the t-shirts
  (3 L × 6.5 + 7 M × 3.5 + 7 S × 1.5 ≈ 55), trimmed for known-target speed.
- **Restart, lifting the MIT core:** **~40 PD** — `F-mcp-ide-bridge`,
  `F-fork-detection`, `F-scheduled-tasks`, and the path/parse core of
  `F-session-browser`/`F-builtin-terminal` mostly evaporate; the dominant
  remaining cost is `F-file-diff-panel` + the renderer UI. Quality is built in.
- **Continue + remediate:** new features cost ≈ their t-shirt (cheap, at the
  margin), but the **debt tax** to reach a clean, testable base is **~30 PD**:
  un-god-objecting `main.js` (~10–15), retrofitting a test harness +
  characterization tests onto untested code (~8–12), de-singletoning
  `db.js`/`mcp-bridge.js` and fixing races/silent-catches/CSP/single-instance
  (~5–8). It ships **zero new user value** and carries **high hazard** —
  refactoring working code you can't yet test (the #46 regression is that
  hazard realised).

| Dimension            | Alt 1 — Continue + remediate | Alt 2 — Restart clean   |
| -------------------- | ---------------------------- | ----------------------- |
| Build features       | ~0 (they exist)              | ~40 PD (lift + build)   |
| Quality / debt       | ~30 PD tax, **may not stick** | included (build clean)  |
| Hazard               | **high** (regress untested)  | medium (parity gap)     |
| Value while underway | yes — keep shipping          | none until parity       |
| End state            | same features, debt paid     | clean, tested, owned    |
| Raw cost vs original | incremental                  | **below** (~40 < ~55)   |

**Reading it your way.** On raw PD the two are close (~40 restart vs ~30
remediate). But your own heuristic — *fixing quality is more expensive and
hazardous than building from scratch* — is exactly what the hazard row encodes:
the ~30 PD remediation is high-risk and may not fully succeed (you can't safely
refactor what you can't test), while the ~40 PD restart is greenfield, low-risk,
and ends with a clean tested base you own. The only real cost a restart carries
that remediation doesn't is the **parity gap** — no shipping to users until the
rebuild reaches feature parity. That gap, not the PD count, is the thing to
weigh.

---

## Appendix A — Empirical effort from git history (v0.0.30)

### Method & caveats

- **Source:** `git log` over the full 146-commit history (2026-03-07 →
  2026-05-20, ~10.5 calendar weeks).
- **Base proxies:** commit count, total churn (insertions + deletions), and
  **distinct active days** touching a feature's files.
- **Adjusted active-days (AAD):** raw active-days corrected for three
  distortions found during review (below). AAD is the column the t-shirt sizes
  and the ~55 PD anchor are built on.
- **Commits ≠ hours.** Relative signal, not a timesheet.

### Three distortions corrected (the anomaly investigation)

1. **Split artifacts (Apr-10 refactor `26fc8a0`).** `git --follow` does not
   trace content extracted into new modules, so files *created* by the split
   show `first ≈ 2026-04-10` and undercount. Confirmed for `F-fork-detection`
   (`session-transitions.js` was born in the split — its raw "1 day" is
   meaningless; adjusted up by complexity).
2. **Shared infrastructure inflation.** `db.js` holds FTS **and** metadata
   **and** settings; its 7 active-days were wrongly attributed wholesale to
   `F-fts-search`. Split across the three consumers → each adjusted down.
3. **Genuine bursts vs. squash.** `F-mcp-ide-bridge` was checked for a squash
   merge — it was **not**: commit `8905896` added the 453-line bridge in one
   real same-day commit (+`1f5dc48` close_tab). Its low active-days are *real*:
   built in ~1–2 days by someone who knew the protocol.

### Per-feature signal (sorted by AAD)

| F-ID                     | Raw days | Churn | AAD | Size |
| ------------------------ | -------- | ----- | --- | ---- |
| `F-session-browser`      | 10       | 1833  | ~8  | L    |
| `F-file-diff-panel`      | 6        | 2484  | ~6  | L    |
| `F-builtin-terminal`     | 5        | 563+  | ~6  | L    |
| `F-activity-stats`       | 4        | 837   | ~4  | M    |
| `F-packaging-ci`         | 4        | 159   | ~4  | M    |
| `F-fts-search`           | 7\*      | 432   | ~3  | M    |
| `F-scheduled-tasks`      | 3        | 683   | ~3  | M    |
| `F-jsonl-viewer`         | 3        | 669   | ~3  | M    |
| `F-fork-detection`       | 1†       | 198   | ~3  | M    |
| `F-status-notifications` | —‡       | —‡    | ~3  | M    |
| `F-settings`             | 3        | 410   | ~2  | S    |
| `F-plans-memory`         | 3        | 291   | ~2  | S    |
| `F-session-grid`         | 2        | 419   | ~2  | S    |
| `F-mcp-ide-bridge`       | 1        | 489   | ~2  | S    |
| `F-app-shell`            | —‡       | —‡    | ~2  | S    |
| `F-session-metadata`     | 3\*      | 63    | ~1  | S    |
| `F-auto-update`          | —‡       | —‡    | ~1  | S    |

`+` churn understated (also lives in `main.js`). `*` raw days inflated by
shared `db.js`. `†` split artifact. `‡` no isolated file — effort buried in
`main.js`. **Σ AAD ≈ 55 PD** — the original build cost, and the ceiling for any
restart.

### The monoliths (unattributable / shared early effort)

| File      | Commits | Churn | Span                  |
| --------- | ------- | ----- | --------------------- |
| `app.js`  | 50      | 9948  | 2026-03-07..05-20     |
| `main.js` | 39      | 3631  | 2026-03-10..05-12     |

Most pre-split feature work was born here. The AAD figures for
`F-status-notifications`, `F-app-shell`, `F-auto-update`, and the early renderer
features draw on this ~13.5k churn that no per-feature file can cleanly claim.

### Authorship

Ali Basiri = **131 / 146** commits (90%). Others: Naved Rangwala 3,
LIS-Benjamin 3, Alex Hazanov 3, HaydnG 2, AliFozooni 2, Oswaldo Alvarez 1,
dtannen 1. Effectively a solo project — which makes the ~10.5-week, ~55-PD
full-scope delivery notable, and makes the bus-factor the real risk.

### Synthesis

1. **A restart can't cost more than ~55 PD, and lands ~40 with lifting.** The
   moat (`F-mcp-ide-bridge`) and the other reverse-engineered modules were the
   expensive *knowledge*, and that knowledge is already captured as liftable
   MIT code — so in a restart their cost mostly evaporates.
2. **The UI is the irreducible cost.** `F-file-diff-panel` was the largest real
   build and isn't liftable; budget the restart around it.
3. **The decision is quality, not PD.** Continue+remediate (~30 PD, high
   hazard, no new value, may not stick) vs restart (~40 PD, low hazard, clean
   owned base). The PD gap is small; the hazard gap is the point — and the only
   cost unique to restart is the **parity gap** before users get the rebuild.
4. **No quality harness ever existed** (one test, no CI gate), which is what
   makes Alt-1 remediation hazardous: there is no safety net under the refactor.
