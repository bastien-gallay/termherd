# Switchboard — Non-Functional Requirements (as-built)

**Date:** 2026-05-27
**Version reviewed:** v0.0.30
**Scope:** what this version actually does for signing, distribution,
updates, CI/CD, security, performance, and reliability — facts only. Gaps are
flagged; the gap-bridging evaluation is the reader's.

## Packaging & distribution

- **Builder:** electron-builder. `appId: ai.doctly.switchboard`,
  `productName: Switchboard`.
- **Publish target:** GitHub, `doctly/switchboard`.
- **Bundled files:** `*.js`, `workers/**`, `public/**`, `node_modules/**`.
- **Platform matrix:**

| OS | Targets | Arch |
| --- | --- | --- |
| macOS | dmg, zip | arm64 + x64 |
| Windows | nsis | x64 + arm64 |
| Linux | AppImage, deb | x64 + arm64 |

- **DMG:** custom background + `/Applications` symlink layout.
- **NSIS:** `oneClick: false`, install dir changeable.
- **Native modules:** `better-sqlite3` and `node-pty` are `asarUnpack`ed
  (must run from disk, not the asar archive). Requires platform build tools
  (Xcode CLT / build-essential+python3 / VS Build Tools).

## Code signing & notarization

- **macOS:** `hardenedRuntime: true`, `notarize: true`,
  `gatekeeperAssess: false`. Signing via `CSC_LINK`/`CSC_KEY_PASSWORD` or
  Keychain auto-discovery. Notarization needs `APPLE_ID`,
  `APPLE_APP_SPECIFIC_PASSWORD`, `APPLE_TEAM_ID`.
- **Entitlements** (`build/entitlements.mac.plist`, applied + inherited):

| Entitlement | Reason | Trade-off |
| --- | --- | --- |
| `allow-jit` | V8 / native modules | — |
| `allow-unsigned-executable-memory` | node-pty / better-sqlite3 | weakens hardened runtime |
| `allow-dyld-environment-variables` | env injection for spawned shells | broadens attack surface |
| `disable-library-validation` | load unsigned native `.node` | lets unsigned libs load in-process |

These four are the standard "native-modules-in-Electron" set, but they
materially relax the hardened runtime — worth a conscious decision in any
rebuild.

- **Windows:** `CSC_LINK`/`CSC_KEY_PASSWORD` for OV/EV (not wired in CI by
  default).
- **CI escape hatch:** `CSC_IDENTITY_AUTO_DISCOVERY=false` for unsigned
  artifact builds.

## Auto-update

- `electron-updater` ^6.3.0, GitHub provider.
- `autoDownload: true`, `autoInstallOnAppQuit: true`.
- Active only when `app.isPackaged` (or `FORCE_UPDATER`).
- Checks on launch and **every 4 hours** (`setInterval`, 4·60·60·1000).
- UX: background download → "ready" toast → restart now or on next quit.

## CI/CD (`.github/workflows/build.yml`)

- **Triggers:** push tags `v*`, and `pull_request` to `main`.
- **Build job:** 4-way matrix (macos-14, windows-latest, ubuntu-latest x64,
  ubuntu-24.04-arm arm64), `fail-fast: false`. Node 20, npm cache,
  Python 3.12 with setuptools (node-gyp fix), `npm ci`, bundle CodeMirror,
  `electron-builder --<platform> --publish never`, upload artifacts.
- **Linux is split per-arch on purpose** (documented inline, see #18): a single
  multi-arch+multi-target invocation rebuilt native modules for the wrong arch.
- **Publish job:** only on `v*` tags; downloads all artifacts, creates a
  **draft** GitHub release via `gh release create`.
- **Gaps:**
  - ❌ **No test step** — CI never runs `npm test` (the one existing test is
    unenforced).
  - ❌ **No lint step** — no ESLint/Prettier/markdownlint in CI or repo.
  - ⚠️ **Node version drift** — CI pins 20; untracked `.node-version` says 24;
    `package.json` has no `engines`. README says "Node 20+".

## Security posture

- ✅ `contextIsolation: true`, `nodeIntegration: false`; `contextBridge` is the
  sole renderer API surface (`window.api`).
- ⚠️ **`sandbox` not set explicitly** on the BrowserWindow (relies on Electron
  default; worth pinning intentionally).
- ❌ **No Content-Security-Policy** — no CSP meta tag and no
  `onHeadersReceived` header injection. The renderer loads `node_modules`
  directly via `<script>`.
- ❌ **No single-instance lock** (`requestSingleInstanceLock` absent) — a second
  instance would race on the SQLite DB and `~/.claude/ide` lock files.
- External links handled explicitly (`open-external` IPC → shell).
- Reads OAuth credentials from the macOS Keychain (`claude-auth.js`) — handle
  with care in any rebuild.

## Performance

- **Off-thread scanning:** project scan runs in a `worker_threads` Worker
  (`workers/scan-projects.js`) to keep the main thread responsive.
- **Terminal rendering:** xterm with the WebGL renderer (`@xterm/addon-webgl`,
  added v0.0.26) and unicode-graphemes addon for emoji/wide chars; logs WebGL
  init failures instead of swallowing them.
- **Idle CPU:** terminal cursor blink disabled to cut idle CPU (#36).
- **Search:** SQLite FTS5 index; case-insensitive; title-only toggle.
- **Renderer diffing:** morphdom (no virtual DOM framework).
- **Bounded reads:** session text content truncated (~8000 chars) when
  digesting JSONL.

## Data & storage

- App data: `~/.switchboard/switchboard.db` (better-sqlite3, WAL mode). Auto-
  migrates from legacy `~/.claude/browser/*`.
- Reads `~/.claude/projects/**` and `~/.claude/` (CLAUDE.md, plans).
- Writes IDE lock files to `~/.claude/ide/`.
- `.gitignore` excludes `*.db`, `*.db-wal`, `*.db-shm`, `dist/`,
  `codemirror-bundle.js`.

## Observability

- `electron-log` ^5.3.0 to file + console. Level: `info` when packaged,
  `debug` in dev. Auto-updater logs through it.
- ⚠️ Inconsistent in app code: some errors use `log.error`, some
  `console.error`; ~12 empty `catch {}` blocks swallow errors silently.

## Reliability / lifecycle

- Lifecycle hooks: `app.whenReady`, `window-all-closed`, `before-quit`,
  `will-quit` (PTY cleanup on quit).
- Window bounds persisted and restored (with macOS clamp workaround).
- ⚠️ **Reliability gaps for a rebuild to address:** no single-instance lock;
  MCP `openDiff` promise has no timeout (hangs if the renderer never replies);
  acknowledged race on `session.realSessionId` between transition detection and
  PTY exit; silent catch blocks hide partial failures.

## Versioning & release

- SemVer in `package.json` (0.0.30). Release is **tag-driven**
  (`git tag vX.Y.Z && git push --tags`), producing a draft GitHub release.
  Local `npm run release` publishes directly with `GH_TOKEN`.

---

## NFR scorecard (for the gap eval)

| Area | State |
| --- | --- |
| Packaging (multi-platform) | ✅ strong |
| macOS signing + notarization | ✅ wired |
| Auto-update | ✅ solid, off-the-shelf |
| CI build matrix | ✅ strong |
| CI test/lint gate | ❌ absent |
| Renderer security baseline | ✅ isolation; ⚠️ no CSP, no explicit sandbox |
| Single-instance safety | ❌ absent |
| Observability | 🟡 present but inconsistent |
| Reliability hardening | 🟡 gaps (timeouts, races, silent catches) |
| Versioning/release flow | ✅ clean, tag-driven |
