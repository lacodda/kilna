# 0016 — An agent outside the window proposes too

Date: 2026-09-11
Status: Accepted

## Context

The assistant panel runs the owner's Claude Code CLI inside kilna, and since
v0.28 it has worked under one rule: the assistant proposes, a person applies.
An answer becomes a version through *insert as version*; a score arrives as
a fenced block the panel turns into numbers with an *apply* button. Nothing
the model says reaches the database until someone clicks.

The owner asked for the other direction (decision 2026-09-10, п.10 of the
patch): an interface for neural networks, so that a Claude Code session
running anywhere — in a project directory, over a vault, in a terminal —
can read the workspace and put things into it, without the panel. Three
forms were on the table: a REST port, a headless CLI, an MCP server. And
one question underneath: does an agent outside the window get to write?

## Decision

**The same binary serves MCP on stdio: `kilna --mcp`.** JSON-RPC 2.0, one
line per message, no port and no key; the client starts the process and the
process dies with the pipe. The same protocol `rigger` serves the line's
records over, so the two register in Claude Code the same way and an agent
meets one convention. A REST port would mean a listener, an address and a
credential for a single-user desktop tool; a CLI would mean every agent
learning a private grammar. MCP is what the agents already speak.

**Reading is open; writing is a proposal.** The tools read everything the
card shows — the catalogue, a work, a body, the scores, the calendar, the
notes, a search — and can write nothing directly. `propose_version`,
`propose_score` and `propose_note` put a message into a chat on the work,
named after the client, carrying the proposal in its `meta` exactly as the
panel's own answers do. The buttons that apply those apply these. The owner
chose this over read-only (a session that can read and not answer is half
a conversation) and over direct writes (which would break the one rule
that makes the assistant safe to leave running).

**Proposals are messages, not a new table.** A proposal is a thing someone
said about a work; the chat is where such things already live, with the
apply buttons, the copy button, the history of what was proposed and when.
A second inbox would be a second place to look. The journal gets a line
per proposal, so the bell counts it and the dashboard names it.

**The headless mode resolves the data directory itself.** Tauri's
`app_data_dir` needs an app handle, and there is no app; `db::default_data_dir`
applies the same three platform rules by hand, and `--workspace <dir>`
overrides it for tests and second workspaces.

## Consequences

- Two processes on one SQLite file, both in WAL mode. Writes from the
  server are single inserts; the window learns of them on its next fetch —
  the transcript and the chat lists refetch on an interval while open, and
  the bell already did.
- The chat title is the client's name from `initialize`. A client that gives
  none lands in a chat called *An agent*; a chat is created once per client
  per work and reused.
- A proposal from outside carries `meta.source = "mcp"` and `meta.client`,
  and the panel says who proposed it. Applied, it is an ordinary version,
  score or note with no mark of its origin — the same choice ADR-less v0.28
  made for the panel's own proposals: the note carries the "why", the row
  does not carry the "who".
- Nothing in the server touches the operations log: a chat message was never
  logged, and a proposal is one. What a person applies is logged as their
  own operation, as before.
- MCP-planned work for 1.x (`v1.13`) becomes a matter of coverage: tools for
  what appears after this version, not a server to build.
