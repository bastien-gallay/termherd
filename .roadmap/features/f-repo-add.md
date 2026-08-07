+++
id = "F-repo-add"
type = "feature"
area = ["sidebar", "sessions"]
status = "done"
target = ["Should"]
+++

Declare a repository in the sidebar, before it has any session.

**#279.** A sidebar group is born from a *session*: the scan walks
`~/.claude/projects`, so a repository you have never opened Claude in cannot
appear, and the `$` / 🤖 launch buttons beside a project header — exactly the
gesture you want for it — are out of reach. This adds the missing half.

The change is one line of model and one of consequence. `RepoMeta`, already
keyed by real project path in `~/.termherd/metadata.json` and already written
to grow, gains a `declared` flag orthogonal to its star; and the sidebar stops
being a pure function of the scan:

```text
visible = (discovered ∪ declared) − removed − presentation
```

Only the first two terms are in scope. The predicate is written **once** so
[F-repo-remove](#f-repo-remove) and the ephemeral presentation mode (#265) can
each drop their term into it rather than growing a second visibility rule.

Three details carried the risk. The union is keyed on the path, so a declared
repository that gains its first session stops being empty instead of doubling.
And "declared repositories sort to the top until they have a session" meets the
repo star from [F-favorites](#f-favorites), so the two became one ordered key
(starred → declared-and-empty → recent activity) rather than two competing
sorts of the same list.

**The normalisation was designed wrong and corrected before a line was
written**, which is the part worth keeping. The plan said a subdirectory
resolves to its `repo_root()` — but the scan does not key on `repo_root()` at
all: it keys on the session's `cwd` passed through `resolve_worktree`, and
`repo_root()` *stops at a linked worktree*, whose `.git` is a file. Declaring a
worktree that way would have filed it under a path the scan never produces, and
the repository would have appeared twice — the exact duplicate-sidebar class
FR1 pins, reintroduced by the rule meant to prevent it. The two compose
instead, in order: file → parent, canonicalise, `repo_root()`, then
`resolve_worktree`. One public function in `scan`, called by the declaration
path and by the walk, because two crates each holding their own idea of "the
same repository" would drift silently. Its test asserts against the walk's own
output rather than a hand-written expectation, so a drift in *both* rules still
fails.

Two gestures, converging on a single event so there is one path to test: a `+`
button opening the native folder dialog (`rfd`, `xdg-portal` on Linux so no GTK
is pulled in), and a folder dropped on the window. Neither is exercisable
headless, which is the argument for the convergence.

The agent surface shipped with it: `ProjectSnapshot.declared`, so the ⌘⇧S
capture and the MCP `snapshot` report it from one model, plus `add_repo`
(answering with the key it kept, since the caller may have passed a worktree)
and `forget_repo` (answering whether the row survived on its sessions).
`add_repo` is the first MCP tool to write outside the workspace — into the
overlay, never `settings.json`, so the file-only enumerations in the book stay
true.

One cleanup fell out of it. The snapshot restated the sidebar's filter and
order in its own words under a doc-comment asking the next reader to keep the
two in step; both now go through one `sidebar_row_shown` / `sidebar_row_order`
pair, and a test walks three searches asserting the snapshot's rows *are* the
rendered ones.

Adjacent: [F-repo-view](#f-repo-view) (#148) takes the other end — this is
about a repository *existing* in the sidebar, that one about *viewing* it.
