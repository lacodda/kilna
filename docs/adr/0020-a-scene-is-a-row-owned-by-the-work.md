# 0020 — A scene is a row owned by the work, with prompt blocks keyed by the profile

Date: 2026-09-12
Status: Accepted

## Context

A video is made in scenes: a number, the part of the text it plays
against, a few seconds of the timeline, a kind of shot, a description, and
the prompts a generator is given for it — a still frame, an animation, a
negative. The predecessor kept a scene as a markdown file with a fixed
scaffold that was edited whole; of 2727 such files, 700 were one save away
from corruption, because nothing stopped a stray keystroke from landing in
the scaffold. It also kept a second copy of the storyboard as a text role
of the work, so the two disagreed.

Three questions had to be settled. Who owns a scene — the work, a release
of it, or a version of it. How the kind of shot and the prompt blocks are
named — free text, or the profile's words. Where the context every scene
shares (the hero, the palette, the lens) lives.

## Decision

**A scene is a row of `scene`, owned by the work** (decision of
2026-09-11). Not by a release: a second attempt at a video is a second
video made from the same donor (ADR 0019), and its scenes go with it. Not
by a version: the storyboard is not a body with revisions, it is a set of
rows each edited in place, with a field clock per field (ADR 0012) and its
own tombstone. The number is a plain integer with no uniqueness on it —
renumbering with a shift is its own stage (v0.67), and a scene restored
from the trash must not be refused because its number was taken meanwhile.

**The kind of shot and the prompt blocks are words of the work's kind.**
A kind gains two optional lists (format 2 is not broken: a document
without them is the same document): `shot_types`, so the board can be
narrowed to "every detail"; and `scene_blocks`, the prompts a scene
carries, each with a key, a label and a hint. A scene stores the blocks as
one JSON object keyed by block; a shot type or a block key the kind does
not name is refused on write, so a scene typed as `closeup` when the
vocabulary says `close` cannot exist. A kind that names neither has no
storyboard and no Scenes tab — a song does not.

**Editing happens inside a block, never in the scaffold.** The screen
draws the fields and a box per block with its own copy button; the scaffold
is the schema's. The blocks are saved as a set, so the log's `before` holds
the set as it was and an undo puts the set back.

**The shared context is a version role, `context`**, on the video and
short kinds. It is a body with revisions, read by the assistant's `text`
tool and written by the export like any other role, and it is edited where
bodies are edited; the Scenes tab shows the current one and opens that
lane. Not a column on the work (a second meaning for the table), not an
overview field (those are the profile's, shown on every kind).

Rejected: a table per block (the trash captures by the column that ties a
row to the entity deleted, and a second `work_id` on the block would be the
duplicate ADR 0001 forbids); a status column for the frame (it derives from
the assets, v0.65); scenes as sections of a markdown body (the corruption
above, and a second truth beside the table).

## Consequences

- Migration 0016; `scene.create` and `scene.update` in the operations log,
  deletion through the trash as `scene`; `scene` in the tables a replay
  must rebuild. The trash captures a work's scenes with the work.
- The card gains a *Scenes* tab for kinds that have a storyboard: the
  shared context, a strip to narrow by kind of shot, one card per scene,
  *Add scene* after the last.
- The profile editor lists a kind's kinds of shot and prompt blocks beside
  its statuses; keys are given in the document, labels are renamed there.
- The shipped Studio profile gives the video and short kinds six kinds of
  shot, three blocks and the `context` role; a workspace that already has
  those kinds gains them at the next start, on the terms of ADR 0017 —
  only where the stored copy names none.
- The MCP `workspace` tool lists `shot_types` and `scene_blocks` per kind,
  `work` counts scenes, and a `scenes` tool reads the board. The export
  writes a *Scenes* section.
