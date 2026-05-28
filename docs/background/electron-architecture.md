# Switchboard — Architecture

**Date:** 2026-05-27
**Version reviewed:** v0.0.30
**Purpose:** reference map for fork / replatform decisions.

## One-paragraph summary

Switchboard is an Electron app with three execution contexts bridged by IPC.
It does not store its own conversation data — it is a **reader and controller
of Claude CLI's own files**. It scans `~/.claude/projects/<encoded>/*.jsonl`,
caches a digest in a local SQLite DB, runs sessions in embedded `node-pty`
terminals, and impersonates an IDE to the CLI over per-session WebSocket MCP
servers so that file opens/diffs land in an in-app panel.

---

## Execution contexts

```text
┌─────────────────────────────────────────────────────────────────┐
│  MAIN PROCESS  (main.js, 1461 LOC — the orchestrator)             │
│                                                                   │
│   node-pty sessions (activeSessions Map)                          │
│   SQLite DB (db.js)        MCP WebSocket servers (mcp-bridge.js)   │
│   file watching            auto-updater       scheduler           │
│   ~40 ipcMain handlers across ~10 domains                         │
└───────────────▲───────────────────────────────────┬──────────────┘
                │  ipcMain.handle / .on / webContents.send
                │                                     │
        ┌───────┴───────┐                             │
        │  PRELOAD       │  contextBridge → window.api │
        │  (preload.js)  │  the ONLY renderer↔main path│
        └───────▲────────┘                             │
                │  window.api.*                        ▼
┌───────────────┴──────────────────────────────────────────────────┐
│  RENDERER  (public/, plain JS, no framework, <script> load order) │
│                                                                   │
│   app.js (entry/wiring)   sidebar  grid-view  jsonl-viewer        │
│   file-panel + codemirror  terminal-manager (xterm)              │
│   stats-view  settings-panel  dialogs  plans-memory-view          │
│   morphdom for DOM diffing                                        │
└───────────────────────────────────────────────────────────────────┘
```

### 1. Main process — `main.js` (~51 KB)

Owns all privileged state and side effects: PTY lifecycle (`activeSessions`
Map), the SQLite DB, MCP servers, file watching, the auto-updater, the
scheduler, and ~40 IPC handlers. It is the central orchestrator and the
project's biggest structural liability (see "Known structural risks").

Cross-cutting modules are wired via a **dependency-injection convention**:
`module.init(ctx)` receives a shared context object (`PROJECTS_DIR`,
`activeSessions`, `getMainWindow`, `log`, db functions, …). Used by
`session-cache.js`, `session-transitions.js`, `schedule-ipc.js`. This is the
intended pattern for new main-process modules.

### 2. Preload — `preload.js`

The sole bridge. Exposes `window.api.*` via `contextBridge` with
`contextIsolation: true` / `nodeIntegration: false`. Every renderer↔main
interaction is declared here as an `invoke` (request/response), `send`
(fire-and-forget), or `on` (main→renderer event). Adding a cross-boundary
feature means a method here **and** a handler in `main.js`.

### 3. Renderer — `public/`

Plain JS, **no framework and no bundler** (except CodeMirror, bundled via
esbuild → `codemirror-bundle.js`). Scripts load in order through `<script>`
tags in `index.html`; load order is load-bearing. `app.js` is the entry and
wiring. Uses `morphdom` for DOM diffing and `xterm` for terminals.

---

## Main-process module map

| Module | Responsibility | DI `init(ctx)` |
| --- | --- | --- |
| `main.js` | orchestrator, IPC, PTY, window, updater | host |
| `db.js` | SQLite cache, metadata, FTS, settings | no (singleton) |
| `session-cache.js` | scan coordination + cache persistence | yes |
| `workers/scan-projects.js` | off-thread folder scan (Worker) | workerData |
| `session-transitions.js` | fork / plan-accept detection, re-key MCP | yes |
| `derive-project-path.js` | recover real cwd from JSONL; collapse worktrees | pure |
| `encode-project-path.js` | forward path→folder encoding (CLI-compatible) | pure |
| `read-session-file.js` | parse one JSONL → session digest | pure |
| `mcp-bridge.js` | per-session WebSocket MCP / IDE server | no (singleton) |
| `shell-profiles.js` | shell discovery + WSL path translation | module cache |
| `schedule-runner.js` | frontmatter + cron parse, run scheduled cmds | partial |
| `schedule-ipc.js` | IPC glue for scheduling | yes |
| `claude-auth.js` | read Claude OAuth creds for usage stats | no |
| `folder-index-state.js` | folder mtime digest (the one tested module) | pure |

---

## External data dependencies

Switchboard is a guest in Claude CLI's filesystem:

| Path | Mode | Used for |
| --- | --- | --- |
| `~/.claude/projects/<enc>/*.jsonl` | read | session discovery + transcripts |
| `~/.claude/ide/*.lock` | write | MCP IDE registration (per session) |
| `~/.claude/` (CLAUDE.md, plans) | read/write | memory & plan editing |
| `~/.switchboard/switchboard.db` | read/write | cache, metadata, FTS, settings |

`db.js` auto-migrates from older `~/.claude/browser/*` DB locations. The
folder-name encoding under `projects/` is **lossy**, which is why
`derive-project-path` reads the `cwd` field inside the JSONL to recover the
real path — the recurring source of duplicate-sidebar bugs (#41, #44).

---

## Key data flows

### Session discovery → display

1. `session-cache` (or the `scan-projects` Worker) reads each project folder.
2. For each `*.jsonl`: `derive-project-path` → real path;
   `read-session-file` → digest (summary, counts, titles).
3. Digests are upserted into SQLite + FTS.
4. `buildProjectsFromCache` groups by real path, sorts, filters archived.
5. Result pushed to renderer (`projects-changed`); sidebar morphdom-diffs it.

### Launching a terminal

1. Renderer calls `openTerminal(id, projectPath, isNew, opts)`.
2. Main resolves the effective shell profile, builds the Claude command,
   spawns `node-pty` with a cleaned env (`cleanPtyEnv`), starts an MCP server.
3. PTY `onData` is parsed for OSC sequences (title, busy/idle, notifications)
   and streamed to the renderer (`terminal-data`); xterm renders it.
4. `onExit` cleans up `activeSessions` and the MCP server.

### IDE emulation (MCP)

1. Each PTY gets a WebSocket MCP server + lock file in `~/.claude/ide`.
2. The CLI discovers Switchboard as its IDE and routes `openDiff`/`openFile`
   over JSON-RPC.
3. Main forwards to the renderer (`mcp-open-diff`); the file panel shows a
   CodeMirror diff; the user accepts/rejects/edits.
4. The renderer replies (`mcp-diff-response`); main resolves the RPC promise.

### Fork / plan-accept detection

`session-transitions` watches for newly created JSONL files, reads their head
(forkedFrom/plan signals) and the old session's tail (ExitPlanMode marker),
matches within ~30s, then re-keys the MCP server to the new session id.

---

## Architectural decisions & trade-offs

| Decision | Upside | Cost |
| --- | --- | --- |
| Read Claude's own files (no own data model) | always in sync, zero import | tightly coupled to CLI's undocumented format |
| Plain-JS renderer, no framework/bundler | zero build for UI, fast iteration | `<script>` order fragility, no component model, manual DOM via morphdom |
| SQLite cache + Worker scan | fast cold start, off-thread I/O | cache invalidation + duplicate-path bugs |
| `init(ctx)` DI for main modules | testable, decoupled where applied | applied inconsistently; globals still reached directly |
| Per-session WebSocket MCP server | true IDE emulation | lifecycle/lock-file complexity, no RPC timeouts |
| `main.js` as orchestrator | one place to wire everything | 1461-LOC god-object |

---

## Known structural risks (from the standards review)

- **God-object `main.js`** — ~40 handlers across ~10 domains in one file; the
  `open-terminal` handler alone is ~330 LOC. The documented remedy (extract
  `session-ipc` / `project-ipc` / `file-ipc` via `init(ctx)`) is not yet done.
- **Singletons with require-time side effects** — `db.js` opens SQLite and
  creates `~/.switchboard` at import; `mcp-bridge.js` holds a module-global
  `servers` Map. Both resist isolation and testing.
- **Predictability** — ~64 empty `catch {}` / `catch (e) {}` blocks across
  `main.js` + sibling modules; logging split between `log.*` and `console.*`
  (23 vs 10 in `main.js` alone); an acknowledged race on
  `session.realSessionId`; `handleOpenDiff` awaits a renderer reply with no
  timeout.
- **No single-instance lock** — two app instances would race on the same DB
  and `~/.claude/ide` lock files (see NFR doc).
- **Testability cliff** — clean pure modules (path/JSONL/cron) are untested but
  trivially testable; everything in `main.js`/`mcp-bridge.js` needs extraction
  or fakes first.

---

## The crown-jewel asset: the CLI-format adapter

The most valuable thing here is **not** the orchestration code — it is the
reverse-engineered knowledge of Claude CLI's *undocumented* surface:

- the lossy `projects/<encoded-path>/` folder encoding and the `cwd`-recovery
  trick (`derive-project-path` / `encode-project-path`);
- the JSONL transcript shape and how to digest it (`read-session-file`);
- the fork / plan-accept signals (`session-transitions`);
- the OSC sequences parsed off PTY output (title / busy-idle / notifications,
  currently inline in `main.js`);
- the MCP IDE handshake and `~/.claude/ide/*.lock` protocol (`mcp-bridge`).

Today this knowledge is **correct but scattered** across modules and inline
code. The recurring duplicate-sidebar bugs (#41, #44) are symptoms of it being
implicit. Any of the three paths below should first **consolidate it into one
documented, versioned anti-corruption layer backed by captured fixtures** —
this is the asset most likely to break on a CLI update and the one worth
hardening regardless of language or framework.

---

## Decision matrix — refactor vs. restart vs. replatform

| | **Refactor in place** | **Restart from scratch** | **Replatform to Rust (Tauri)** |
| --- | --- | --- | --- |
| Effort | Low–medium | High | Very high |
| Risk | Low | Medium | High |
| Fixes the god-object | Yes (the point) | Yes | **No** — bad design is bad in Rust too |
| Fixes `<script>`-order UI fragility | Partly (add bundler/modules) | Yes (framework day 1) | No — webview UI is rebuilt, not improved |
| Rebuilds the renderer | No | Yes (costliest part) | **No** — xterm/CodeMirror stay JS in webview |
| Improves runtime profile (RSS / idle CPU / binary size) | Marginal | No | **Yes — the only thing that justifies it** |
| Preserves the CLI-format knowledge | Yes | Only if deliberately lifted | Must be re-implemented in Rust |
| Justified by | structural risk list below | UI redesign | measured NFRs (see baseline) |

Throughline: **refactor is the default and the prerequisite; restart is
justified only by the UI; Rust is justified only by runtime metrics — and it
leaves the UI in place regardless.**

---

## Path 1 — Refactor in place (ordered, dependency-aware)

Not a flat risk list; the ordering *is* the plan. Each step is the safety net
for the next.

1. **Test the pure core first** — `derive-project-path`, `encode-project-path`,
   `read-session-file`, the cron/frontmatter parser. Zero dependencies,
   trivially testable, and everything else leans on them. (`folder-index-state`
   is currently the *only* tested module.)
2. **Decompose `main.js`** via the existing `init(ctx)` convention — extract
   `session-ipc`, `project-ipc`, `file-ipc`, and pull the ~330-LOC
   `open-terminal` handler into a terminal-lifecycle module.
3. **Kill require-time side effects** — `db.js` opens SQLite + `mkdir`s at
   import; `mcp-bridge.js` holds a module-global `servers` Map. Convert to
   factory / `init(ctx)` so they can be faked.
4. **Fix the two named bug-classes** — add an RPC timeout to `handleOpenDiff`
   (today it awaits the renderer forever) and add
   `app.requestSingleInstanceLock()` (confirmed absent; two instances race the
   DB and `~/.claude/ide` locks).
5. **Unify logging** on `electron-log` and replace the ~64 empty catches with a
   logged swallow at minimum.

## Path 2 — Restart from scratch (keep vs. rebuild ledger)

- **Keep & harden:** the CLI-format adapter (above) — top of the ledger, not a
  footnote.
- **Reaffirm, don't relitigate:** the **stateless-reader design** ("no own data
  model"). Listed as a trade-off with a cost above, but it is the
  architecture's single best property — always in sync, zero import. A restart
  should keep it.
- **Rebuild:** the renderer, framework + bundler chosen on day one (esbuild is
  already in the toolchain for CodeMirror — the `<script>`-order fragility is
  the cheapest thing a clean start removes).

## Path 3 — Replatform to Rust (Tauri)

Two facts the decision usually misses:

1. **Rust does not rebuild the UI for free — and the UI is the expensive part.**
   Tauri's frontend is a system-webview running JS. **xterm.js and CodeMirror
   have no Rust equivalent** and stay in the webview. So a Rust port rewrites
   the cheap-to-keep layer (orchestration) and barely touches the
   expensive-to-rebuild layer (renderer) — the inverse of the restart calculus.
2. **Rust does not fix architecture.** A god-object is just as bad in Rust.
   Refactor (Path 1) *first*; replatform only for the runtime profile.

### Component migration table

| Today (Node / Electron) | Rust / Tauri path | Notes |
| --- | --- | --- |
| `node-pty` | `portable-pty` | WSL path translation must be re-solved |
| `better-sqlite3` | `rusqlite` | straightforward; WAL supported |
| `ws` (MCP server) | `tokio-tungstenite` | gains real async lifecycle + timeouts |
| file watching | `notify` | maps cleanly |
| `electron-updater` | Tauri updater | different signing / manifest flow |
| 40 `ipcMain` handlers + `preload.js` | Tauri commands / events | full bridge rewrite |
| `xterm` / CodeMirror / `morphdom` | **stays JS in webview** | no Rust replacement |

**What Rust buys:** ~10 MB binaries vs Electron's ~150 MB, far lower idle
memory, `Result` types that structurally eliminate the empty-catch problem, and
a real concurrency model (tokio) for the scan / watch / PTY / MCP orchestration.
Motivation hint already in the git history: commit `ae61180` *"disable cursor
blink to cut idle CPU."*

---

## NFR baseline (to measure — the data the Rust decision hinges on)

The Rust question is **unanswerable without these numbers**, and none are
currently captured. Measure on the current Electron build, then re-measure any
prototype:

| Metric | How to measure | Current |
| --- | --- | --- |
| Shipped binary / installer size | `npm run build` → `ls -lh dist/*.dmg` | *TBD (no `dist/` build present)* |
| Idle RSS (main + renderer) | `ps`/Activity Monitor, app open, no active PTY | *TBD* |
| Idle CPU % | same, sample over 60 s | *TBD (the `ae61180` motivation)* |
| Cold-start to first paint | manual stopwatch / perf mark | *TBD* |
| RSS under N live terminals | open 5–10 sessions, sample | *TBD* |

Reference only (not the shipped figure): unpacked `node_modules` —
`electron` ≈ 283 MB, `node-pty` ≈ 63 MB, `better-sqlite3` ≈ 12 MB.
