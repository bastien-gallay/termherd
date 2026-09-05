+++
id = "F-mcp-pointer-terminal"
type = "feature"
area = ["mcp", "terminal"]
status = "todo"
target = ["Could"]
+++

The pointer rung, terminal half: place a mouse event inside a session.

The pointer rung, terminal half: place a mouse event **inside a session's
terminal**, the way `run_in_session` places text there. Filed as #300. Cell
addressed — a terminal is a grid, and a grid is what an SGR report carries — so
the tool is `mouse_in_session(session, kind, col, row, …)`, bounded by the
pane's geometry, answering what the pane did with it (`forwarded` when the
child had mouse reporting on, `selection` when it drove local text selection,
`rejected` out of bounds).

Two gaps close with it. The act→wait→observe loop has no pointer at all, so
every mouse-mode TUI a session hosts — Claude Code's `/diff` and `/resume`,
lazygit, fzf, vim — is unreachable to an agent whose keyboard already works.
And #155 (mouse clicks are never encoded to the child) cannot be verified by
the agent that fixes it: its whole contract is a pointer gesture, and no test
in the tree can produce one. **Blocks #155.**

It shares one seam with that bug and should not be built twice: this rung
introduces the pointer *event path* (a `core::Event` carrying cell coordinates
and button state, through the shell to the focused pane) and exposes it over
MCP, while #155 introduces the SGR encoder in `pty` and the mode gate that
chooses between forwarding and local selection. Local selection is the
observable behaviour before #155 lands, so the rung has a test standing alone.

Sibling of [F-mcp-pointer-chrome](#f-mcp-pointer-chrome), which drives
termherd's own interface rather than a terminal and blocks nothing.
