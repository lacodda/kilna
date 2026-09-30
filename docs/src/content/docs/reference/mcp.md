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
| `workspace` | The active profile's vocabulary: the overview fields (each with the kinds it belongs to and, for a choice, its options), and per kind of work its version roles and how each reads, axes with weights and scales, tiers, statuses, kinds of release (each with the fields a release of it goes out under and the shape of its cover), whether it goes out at all (`goes_out` — a song does not: its clip, audio and shorts do), for a kind with a storyboard its kinds of shot and the prompt blocks a scene carries, the parts of its cover prompt, and whether its works play under a frame; how many works. Read first — a work is judged in its own kind's keys, and every other tool speaks in them. |
| `catalogue` | Every work with its verdict: id, title, kind, status, total and tier, whether the score is stale, releases out and scheduled, when it was last touched. Filter by a substring of the title, a kind, a status. |
| `work` | One card: fields and meta, tags, every version by role (id, revision, label, length, which is current), the latest score with its axes, the releases, how many notes and scenes, the cover prompt, the frame (for a kind that plays under one: the still, the loop as written from its settings, the negative), what it was made from (`sources`, each saying whether the source has moved on since) and what was made from it (`derived`); for a work that never goes out itself — a song — its `publications`: everything made from it, with where each stands, and the release its status stands on. No bodies. |
| `text` | The body of a version: the current one of a role, or a revision by id. Plain roles come back exactly as typed. |
| `scores` | The score history of a work, newest first. |
| `calendar` | Every release with a date, in calendar order; `from` starts at a day. |
| `notes` | Notes, all of them or one work's; `kind` narrows to ideas, phrases or any other kind, `state` to the fresh, used, parked or dropped ones of a material kind. |
| `register` | The register of repeats: every term with its forms, kind, strictness, topic, note and how many works carry it now; or, with `text` or `work`, that text checked — the terms it takes word for word and how often, and the words it leans on within itself. |
| `scenes` | The storyboard of a work, in order: each scene's number, section, seconds, kind of shot, description and prompt blocks. The shared context is the `context` role — read it with `text`. |
| `canon` | The [canon](/kilna/guides/the-canon/): without arguments, every card — id, kind, name, aliases, layer, the work it lives at, how many facts, drafts and live zones — and the kinds of card with their sections. With `card` (an id, or an exact name or alias), that card whole: its facts by section, each with its layer, status, source, time in the world and the tasks that may read it; its relations; where it appears; its description for a picture generator and whether it is stale. `lens` — `cover`, `work` or `public` — narrows the card to what that task may read, by the same rule the window dims by; a card a lens cannot see answers that it does not exist. `query` searches the facts; `timeline` lists the facts dated in the world, earliest first. |
| `search` | Works, versions, notes, facts and replies by text; every hit names its work or its card. |

A work is named by id, or by its exact title. A title two works share is
refused with the ids to choose from, rather than guessed.

## What an agent can propose

Nothing an agent does writes a work, a version, a score, a note or a
scene. It **proposes**, and the proposal lands as a message in a chat named after the
client — *Claude Code*, say — on the work, or on nothing for a work that
does not exist yet. The same buttons that apply the assistant's own
proposals apply these:

| Tool | Where it lands |
| --- | --- |
| `propose_work` | A whole work — title, kind, overview fields, versions by role, a score, notes, for a kind with a storyboard its scenes, and the releases it ships as with the text each goes out under — or, with `work`, a package of those for an existing one. The message shows everything the package would write; **Create the work** or **Apply the package** writes all of it in one click. On a new work the version in the first role becomes current; on an existing one the package's versions wait beside the current, and its scenes go after the last on the board. A `releases` entry names its kind of release, optionally the day it goes out, and `fields` — what it goes out as, by the field keys that kind names. A kind of release the work does not ship refuses the whole package; a field key that kind does not have is named and left out. |
| `propose_scenes` | A storyboard for a video or a short: scenes with their number, section, seconds, kind of shot, description and prompt blocks, in the kind's own words. The message shows the board as a table with every block under it. `change` says what happens to the board that is there: `add` (the default) — **Add to the board** numbers the scenes after the last; `replace` — **Replace the board** rewrites a scene with the same number in place — it keeps its id — sends the rest of the old board to the trash, and creates the numbers nobody held; `revise` — **Revise the scenes** changes only the numbered scenes, only in the fields given (a field left out is kept, `blocks` are set together), and sends nothing to the trash. A kind with no storyboard, an unknown kind of shot or block key, a scene that ends before it starts, a revision without numbers refuse the whole package. |
| `propose_version` | The text of a new version in a role, with a note on what changed. **Insert as version** keeps it, verbatim, under that role and not current; **Choose role…** opens the dialog to change the role, name it or make it current on the way in. |
| `propose_score` | Marks along the kind's axes, checked the way the assistant's own are: unknown axes are named, marks are clamped to the scale. **Apply** writes the snapshot, judged by the agent — its name is the score's rater. |
| `propose_note` | A note, on a work or on nothing in particular. **Add as note** keeps it. |
| `propose_release` | What one release goes out under — its title, description, tags, the comment pinned under it — by the field keys its kind of release names. `release` names it by id; it may be left out when the work has one release. A field key the kind does not have is named and left out. The fields wait in the chat on the work and under the release on its Releases tab, each beside what is written there now; the person takes them one by one or all at once. Nothing is written until they do — unlike the window's own **Release meta** action, which fills the empty fields as its answer lands, an agent outside the window only proposes. A song has no release: propose for its clip, audio or short. |
| `propose_canon` | Cards, facts and relations for the [canon](/kilna/guides/the-canon/). A fact is an `add` to a card (by id, name, or the `handle` of a card proposed in the same call), a `refine` of a fact's id, or a `retire` with its reason; one that contradicts the canon says which fact in `contradicts`. `work` is where the facts were read from: every fact with no other source cites it, with the line in `line`, and a card with `on_work: true` lives at it. The message lists every item under a box, with the facts it contradicts beside it; **Keep all** writes the package, or untick what you do not want and keep the rest. A fact kept from a proposal is a draft — the works read it, a cover and a public text do not — until you settle it; one proposed as `open` stays a live zone. An item the canon has no place for — a kind or a section it does not have, a card it cannot find — is named in the answer and left out. |

An applied proposal stays marked in the chat — *Inserted*, *Scored*,
*Created*, with a link to the work a package made — and cannot be applied
twice. When a chat holds more than one unapplied proposal, **Apply all**
takes them in order, and stops at the first that cannot be applied, saying
which. Every application is written as the same operations you would
write by hand, so undo takes each back on its own.

Each proposal leaves a line in the [history](/kilna/guides/the-history/) —
*Claude Code proposed a version for "Harbour lights"*, *Claude Code
proposed a new work, "Winter road"* — so the bell in the title bar counts
it, and the chat it went into is one click away from any screen through the
assistant's button beside it.

This is the rule the assistant panel has followed since v0.28, applied to an
assistant outside the window: it proposes, you apply. See
[ADR 0016](https://github.com/lacodda/kilna/blob/main/docs/adr/0016-an-agent-outside-the-window-proposes-too.md);
for packages and the mark,
[ADR 0018](https://github.com/lacodda/kilna/blob/main/docs/adr/0018-a-proposal-is-applied-by-the-application-and-marked.md);
for storyboards and what replacing one means,
[ADR 0022](https://github.com/lacodda/kilna/blob/main/docs/adr/0022-a-storyboard-is-proposed-whole-and-replaced-by-number.md).

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

  workspace     → kinds song / video / audio / short; fields bpm, key, …, premise
  propose_work  → "Proposed a new work — “Winter road”, 2 versions (lyrics, style),
                   fields premise, a score on 7 axes. It waits in the chat named
                   after you; one click creates it with everything in it."
```

In kilna: the assistant's button in the title bar opens the chat *Claude Code*
with the whole package rendered — the lyrics in a monospace block, the
style prompt, the premise, the marks — and **Create the work** under it.
One click, and the song is on the catalogue with its lyrics current, its
premise on the overview, its score in the history judged by *Claude Code*.

## A video from a song, end to end

The storyboard of a video is rows the agent can propose like anything
else. The whole road, from the song to a board ready for the generators:

```
> Use kilna: make a video for "Winter road" — plot, context, and a storyboard
  with prompts for every scene.

  workspace       → kind video: roles plot / context / style, shot types wide,
                    medium, close, detail, insert, title; blocks still, motion, negative
  work            → "Winter road", song, lyrics current
  text            → the lyrics

  propose_work    → "Proposed a new work — “Winter road — the clip”, 2 versions
                     (plot, context), 6 scenes. It waits in the chat named after you;
                     one click creates it with everything in it."
```

In kilna: **Create the work** — and the video is on the catalogue with its
plot and its context current, and six scenes on the **Scenes** tab, each
with its seconds, its kind of shot and a still-frame prompt to copy. Then
link it to the song on the **Links** tab as *made from*, or ask the agent
to do it in the same breath.

The same package can plan how it ships. An agent that has just written the
board is the one that knows what the description should say, so making it
propose that separately would be a second round trip for one half of one
thought:

```
> Use kilna: …and plan it as a YouTube release on the 2nd, with a title,
  a description and tags.

  propose_work    → "Proposed a new work — “Winter road — the clip”, 2 versions
                     (plot, context), 6 scenes, 1 release (youtube)."
```

Applying it plans the release with its boxes already filled — see [what a
release goes out as](/kilna/guides/planning-a-release/#what-a-release-goes-out-as).

The board is not final on the first pass. A second round works on the
existing work:

```
> Use kilna: the chorus scenes of "Winter road — the clip" are too static —
  redo the board with more close-ups, and time it to the lyrics.

  scenes          → the six scenes as they stand
  text (context)  → the hero, the palette, the lens
  text (plot)     → the plot

  propose_scenes  → "Proposed a storyboard of 8 scenes to replace the 6 on the
                     board of “Winter road — the clip”. It waits in the chat on
                     the work; one click replaces the board."
```

In kilna: the chat on the video shows the new board as a table, every
prompt block under it, and **Replace the board** with a line saying what
that does. One click: scenes 1–6 are rewritten in place, 7 and 8 are
created. Had the new board been shorter, the scenes beyond it would be in
the trash, and undo walks the whole thing back a scene at a time. To grow
the board instead of redoing it, the agent leaves `change` out and the
scenes go after the last; to rewrite the prompts of scenes 3 and 4 and
nothing else, it sends those two with their numbers and `change: revise`.

The plot and the context are versions, so they take the usual road:
`propose_version` in the `plot` or `context` role, **Insert as version**
on the Versions tab. There is no separate tool for them — a video's plot
is a body with revisions like a lyric.

## Protocol

JSON-RPC 2.0, one message per line, protocol version `2024-11-05`. The
server answers `initialize`, `ping`, `tools/list` and `tools/call`; a
notification gets no answer, an unknown method gets a `-32601` error, and a
tool that fails answers *inside* the result with `isError` so the agent
reads the reason. Nothing but protocol goes to stdout; diagnostics go to
stderr.
