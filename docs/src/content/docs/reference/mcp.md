---
title: MCP server
description: kilna --mcp serves your workspace to an agent over the Model Context Protocol — reading is open, writing is a proposal you apply with one click, a whole work at a time if you like.
---

`kilna --mcp` is the same build without a window: the workspace served to an
agent — Claude Code, or anything else that speaks the
[Model Context Protocol](https://modelcontextprotocol.io) — over standard
input and output. No port, no key, no network: the client starts the process
and talks to it through a pipe, and the process stops when the client hangs
up.

## Registering it

The **Settings** screen shows the exact command for the build you are
running, with a copy button. For Claude Code it is:

```
claude mcp add -s user kilna -- "C:\path\to\kilna.exe" --mcp
```

Quote the path; application directories have spaces in them. `-s user` makes
it every session on the machine rather than the one project you ran the
command in, which is Claude Code's default scope. After that a
Claude Code session anywhere on the machine can read your works and propose
to them — no need to have kilna open, though it is fine if it is.

`--workspace <dir>` points the server at another workspace directory. Without
it, it opens the one the window uses.

## What an agent can read

| Tool | What it answers |
| --- | --- |
| `workspace` | The active profile's vocabulary: the overview fields, and per kind of work its version roles and how each reads, axes with weights and scales, tiers, statuses, kinds of release, and — for a kind with a storyboard — its kinds of shot and the prompt blocks a scene carries; how many works. Read first — a work is judged in its own kind's keys, and every other tool speaks in them. |
| `catalogue` | Every work with its verdict: id, title, kind, status, total and tier, whether the score is stale, releases out and scheduled, when it was last touched. Filter by a substring of the title, a kind, a status. |
| `work` | One card: fields and meta, tags, every version by role (id, revision, label, length, which is current), the latest score with its axes, the releases, how many notes and scenes, what it was made from (`sources`, each saying whether the source has moved on since) and what was made from it (`derived`). No bodies. |
| `text` | The body of a version: the current one of a role, or a revision by id. Plain roles come back exactly as typed. |
| `scores` | The score history of a work, newest first. |
| `calendar` | Every release with a date, in calendar order; `from` starts at a day. |
| `notes` | Notes, all of them or one work's. |
| `scenes` | The storyboard of a work, in order: each scene's number, section, seconds, kind of shot, description and prompt blocks. The shared context is the `context` role — read it with `text`. |
| `search` | Works, versions, notes and replies by text; every hit names its work. |

A work is named by id, or by its exact title. A title two works share is
refused with the ids to choose from, rather than guessed.

## What an agent can propose

Nothing an agent does writes a work, a version, a score or a note. It
**proposes**, and the proposal lands as a message in a chat named after the
client — *Claude Code*, say — on the work, or on nothing for a work that
does not exist yet. The same buttons that apply the assistant's own
proposals apply these:

| Tool | Where it lands |
| --- | --- |
| `propose_work` | A whole work — title, kind, overview fields, versions by role, a score, notes — or, with `work`, a package of those for an existing one. The message shows everything the package would write; **Create the work** or **Apply the package** writes all of it in one click. On a new work the version in the first role becomes current; on an existing one the package's versions wait beside the current. |
| `propose_version` | The text of a new version in a role, with a note on what changed. **Insert as version** keeps it, verbatim, under that role and not current; **Choose role…** opens the dialog to change the role, name it or make it current on the way in. |
| `propose_score` | Marks along the kind's axes, checked the way the assistant's own are: unknown axes are named, marks are clamped to the scale. **Apply** writes the snapshot, judged by the agent — its name is the score's rater. |
| `propose_note` | A note, on a work or on nothing in particular. **Add as note** keeps it. |

An applied proposal stays marked in the chat — *Inserted*, *Scored*,
*Created*, with a link to the work a package made — and cannot be applied
twice. When a chat holds more than one unapplied proposal, **Apply all**
takes them in order, and stops at the first that cannot be applied, saying
which. Every application is written as the same operations you would
write by hand, so undo takes each back on its own.

Each proposal leaves a line in the [history](/kilna/guides/the-history/) —
*Claude Code proposed a version for "Harbour lights"*, *Claude Code
proposed a new work, "Winter road"* — so the bell in the corner counts it,
and the chat it went into is one click away from any screen through the
assistant's floating button.

This is the rule the assistant panel has followed since v0.28, applied to an
assistant outside the window: it proposes, you apply. See
[ADR 0016](https://github.com/lacodda/kilna/blob/main/docs/adr/0016-an-agent-outside-the-window-proposes-too.md)
and, for packages and the mark,
[ADR 0018](https://github.com/lacodda/kilna/blob/main/docs/adr/0018-a-proposal-is-applied-by-the-application-and-marked.md).

## A session, end to end

```
> Use kilna: what is the weakest scored song, and what would you change?

  workspace  → seven axes, tiers HOLD / PIC / CLIP …
  catalogue  → 207 works; "Harbour lights" scored 54, tier HOLD
  text       → the current lyrics
  scores     → 54: hook 5, lyrics 6 …

  The chorus repeats its first line; here is a version with a turn in it.

  propose_version → "Proposed a `lyrics` version for “Harbour lights”.
                     It is in the chat on the work, waiting to be inserted — or not."
```

In kilna: the bell shows one new line, the work's chat *Claude Code* holds
the text with **Insert as version** under it. Insert, or don't.

A new song, whole:

```
> Use kilna: make a song from this idea — lyrics, a style prompt, the premise
  on the card, and your score.

  workspace     → kinds song / instrumental …; fields bpm, key, …, premise
  propose_work  → "Proposed a new work — “Winter road”, 2 versions (lyrics, style),
                   fields premise, a score on 7 axes. It waits in the chat named
                   after you; one click creates it with everything in it."
```

In kilna: the assistant's floating button opens the chat *Claude Code*
with the whole package rendered — the lyrics in a monospace block, the
style prompt, the premise, the marks — and **Create the work** under it.
One click, and the song is on the catalogue with its lyrics current, its
premise on the overview, its score in the history judged by *Claude Code*.

## Protocol

JSON-RPC 2.0, one message per line, protocol version `2024-11-05`. The
server answers `initialize`, `ping`, `tools/list` and `tools/call`; a
notification gets no answer, an unknown method gets a `-32601` error, and a
tool that fails answers *inside* the result with `isError` so the agent
reads the reason. Nothing but protocol goes to stdout; diagnostics go to
stderr.
