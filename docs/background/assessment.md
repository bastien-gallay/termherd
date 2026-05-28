# Switchboard — Daily-Driver Assessment & Strategy

**Date:** 2026-05-27
**Author of this note:** Bastien (user, not project owner)
**Subject:** `doctly/switchboard` — Electron unified view of Claude Code sessions
**Status:** decision pending — see Part 4

> This file lives only in my local checkout. It is excluded via
> `.git/info/exclude` so it can never be pushed to `doctly/switchboard`.

---

## Context

I use Switchboard daily but do not own it. It is young, has a genuinely good
initial feature scope, and appears to be a burst-built internal tool whose
maintenance has cooled. I need to decide how to keep relying on it without being
held hostage by a stalling upstream. Three paths are on the table:

1. Contribute fixes/features upstream via PR.
2. Fork and evolve at my own pace.
3. Replatform — rebuild from scratch using this as the reference design.

Paths 2 and 3 would follow my usual funnel: private experiment → evaluate
pace/results/usefulness → death-match against alternatives → real project →
re-evaluate + death-match → open-source publication.

---

## Part 1 — Standards review (conclusions)

Full review done 2026-05-27 against TDD, Tidy-First, CI/CD, CUPID.

| Standard | Grade | Verdict |
| --- | --- | --- |
| TDD | D | Only 1 test; the highest-value pure functions sit untested. |
| Tidy-First | C+ | Some commits mix structural and behavioral changes. |
| CI/CD | B−/C | Strong delivery pipeline; no test or lint gate in CI. |
| CUPID | C | Clean modules + a DI pattern, but a 1,461-line god-object. |

Key findings:

- **TDD.** One test (`test/folder-index-state.test.js`), and it sleeps 1,100 ms
  of wall-clock time — slow and timing-dependent. Meanwhile a cluster of
  pure/near-pure functions sit uncovered: `encodeProjectPath`,
  `readSessionFile` (title precedence), `cronMatches`/`parseFrontmatter`,
  `deriveProjectPath`/`resolveWorktreePath`, `readNewSessionSignals`. The
  v0.0.30 → #46 cycle was a *behavioral regression and its fix* in exactly this
  untested title-precedence logic — a characterization test would have caught it.
- **CI/CD.** Delivery is strong (4-platform matrix, tag-driven release, signing,
  notarization). But CI never runs the tests or any lint, and Node versions
  drift (CI pins 20, untracked `.node-version` says 24, no `engines`).
- **CUPID.** `main.js` is 1,461 lines / 51 KB owning ~40 IPC handlers across ~10
  domains plus PTY, window, updater, watcher, scheduler. The `init(ctx)` DI
  pattern is good where applied (`session-cache`, `session-transitions`) but
  inconsistent; `db.js` and `mcp-bridge.js` are singletons with require-time
  side effects. The small domain modules (`encode-project-path`,
  `derive-project-path`, `read-session-file`) are genuinely clean.
- **Predictability.** ~12 empty `catch {}` blocks; logging split between
  `log.error` and `console.error`; an acknowledged race around
  `session.realSessionId`.

**Calibration:** "collapse under its weight" overstates it. At ~13k LOC and
2.5 months old, the realistic failure mode is **bug accumulation with no
maintainer**, not architectural implosion. The god-object is annoying but
bounded. The debt matters most to *whoever owns it long-term* — i.e. me, if I
fork.

---

## Part 2 — Project-health signals (decisive context)

- **License: MIT**, Copyright 2025 **Doctly**. Forking *and* replatforming are
  legally clean. I may even lift the proven domain modules into a rebuild with
  attribution. (Note: `package.json` `license` is unset; the LICENSE file
  governs.)
- **Owner is a company, not a lone hobbyist.** Author `Doctly
  <support@doctly.ai>`, repo `github.com/doctly/switchboard`. Doctly is an AI
  document-conversion company. This reads as an internal dev tool they
  open-sourced.
- **Dominant author:** Ali Basiri (44 commits). But **external PRs have been
  merged** — Naved Rangwala, HaydnG, Oswaldo Alvarez. The owner has a track
  record of accepting community contributions.
- **Cadence (confirms the cooling):** Mar 107 → Apr 32 → May 7 commits. First
  commit 2026-03-07, last 2026-05-20 (7 days ago).

**What this changes:** this is not "abandoned hobby project." It is "company
built an internal tool in a burst; it now works well enough for them, so
investment dropped." The consequence for me: future maintenance will track
*Doctly's* needs, which may or may not overlap with mine — and big
architectural cleanup is unlikely, because the god-object works fine for them.

---

## Part 3 — The three options, assessed

### Option 1 — Contribute upstream (PRs)

- **For:** lowest effort; single source of truth; helps everyone; owner has
  merged community PRs before.
- **Against:** depends entirely on upstream responsiveness *now*, in the cooling
  phase. Large architectural PRs (the god-object, the test gap) are exactly what
  a maintenance-mode owner won't merge — high friction, low acceptance odds.
- **Best use:** small, obviously-correct bug fixes (the title-precedence /
  path-encoding class). Not for the structural overhaul.

### Option 2 — Fork and evolve

- **For:** I control velocity; never blocked daily; can still cherry-pick fixes
  upstream. MIT makes it clean. Zero feature-parity risk (I start from working
  code).
- **Against:** I inherit the debt and the architecture — a CommonJS, no-test,
  no-framework Electron app with a god-object. Maintenance burden is mine.
  Divergence pain if upstream revives.
- **Best use:** immediate safety net / daily driver; the incumbent in a
  death-match.

### Option 3 — Replatform

- **For:** clean, testable architecture from day one; I understand every line;
  free choice of stack and license.
- **Against:** reaching feature parity is the big risk. The "good feature scope"
  hides real domain work — PTY session lifecycle, MCP/IDE emulation, JSONL
  transcript parsing, SQLite cache, worktree path collapsing. Re-deriving that
  is where replatforms die.
- **Key insight that de-risks it:** the value is **not the code, it's the
  reverse-engineered domain knowledge** sitting in the small pure modules
  (`encode-project-path`, `derive-project-path`, `read-session-file`,
  `session-transitions`, the MCP protocol in `mcp-bridge`). Those are MIT. Lift
  them as a vendored "core" with attribution and rebuild only the architecture
  around them — that skips the part that kills replatforms.

---

## Part 4 — Recommendation: don't choose yet; run a probe + a spike

The decision hinges on a fact I don't have yet (is upstream still responsive?)
and a comparison I haven't run (clean rebuild vs. inherited debt). My funnel is
built for exactly this — so feed it instead of guessing.

**Phase 0 — Probe upstream (cheap, ~2 weeks, do regardless).**
Open 1–2 small, surgical fix PRs (start with the title-precedence /
path-encoding bugs, each with a characterization test). Their merge latency is
the single most decisive datapoint:

- merged in < 2 weeks → Option 1 is viable for my needs; stay light.
- silence / slow → I need control; commit to Option 2/3.

It is good citizenship *and* a free probe of the bus factor.

**Phase 1 — Fork now as the safety net (Option 2).**
MIT makes it clean. Keeps me unblocked daily, zero parity risk. Rebase on
upstream while upstream lives. This is the incumbent in the death-match.

**Phase 2 — Replatform spike standing on the proven modules (Option 3).**
Vendor the domain core (with attribution); rebuild architecture only —
testable core, clean IPC boundaries, a real renderer framework. Time-box it.

**Death-match:** fork (works, debt-laden) vs. replatform-spike (clean, partial
parity). If standing on the extracted modules gets the rebuild to parity faster
than expected, it wins. If not, the fork is the floor. Winner becomes the real
project; because it's MIT-derived, open-source publication at the end is clean.

**Explicitly do NOT:** mount a heroic refactor-PR campaign against the
god-object in someone else's cooling repo. Wrong tool for the debt.

---

## Open questions only I can answer

- How mission-critical is this to my daily flow? (Mission-critical → justify the
  fork's control now. Nice-to-have → contribute and wait.)
- How much divergence from *Doctly's* roadmap can I tolerate before control
  beats convenience?
- What is my appetite for owning an Electron app's maintenance vs. shipping my
  actual work?

---

## Appendix — evidence snapshot (2026-05-27)

- `package.json` version 0.0.30; `node --test` → 1 test, passes (~1.2 s).
- CI `.github/workflows/build.yml`: builds on PR + tags; **no test/lint step.**
- `main.js` = 1,461 LOC / 51 KB. Next largest root module: `mcp-bridge.js` 487.
- No ESLint/Prettier/markdownlint/editorconfig/husky configs present.
- LICENSE: MIT, Copyright 2025 Doctly. Remote: `git@github.com:doctly/switchboard.git`.
- Commit cadence: 2026-03 = 107, 2026-04 = 32, 2026-05 = 7. Last commit 2026-05-20.
