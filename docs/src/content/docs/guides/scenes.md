---
title: Scenes
description: The storyboard of a video — one row per scene with its seconds, its kind of shot and a prompt block per generator, owned by the work.
---

A video is made in scenes. In kilna a scene is a row of the work's
storyboard: a number, the part of the text it plays against, a few seconds
of the timeline, a kind of shot, a description, and a prompt block for each
thing a generator is asked for — a still frame, an animation, a negative.
The scaffold is fixed; you type inside the fields and the blocks, never
around them.

Scenes belong to the **work** — not to a release, not to a version. A
second attempt at a video is a second video
[made from the same song](/kilna/guides/made-from/), with its own board.

## The Scenes tab

A work whose kind has a storyboard — in Studio, a *Video* or a *Short* —
opens a **Scenes** tab on its card. A song has none: the tab is for kinds
whose profile names kinds of shot or prompt blocks (see
[the profile document](/kilna/reference/profile-document/#shot_types-and-scene_blocks)).

From top to bottom:

- **Context** — what every scene shares: the hero, the palette, the lens.
  It is the current version of the `context` role, a body like the plot,
  so it has revisions and the assistant can read it with `text`. The
  button opens it on the Versions tab; the Scenes tab shows it.
- **A strip of kinds of shot** with counts — *All · Wide · Close-up ·
  Detail…* Click one to see only those scenes: "show me every detail" is
  one click, and the chip that is on turns off.
- **One card per scene.** The number, the section (*intro*, *verse 1*,
  *chorus* — free text until the text's own markup gives the board its
  frame), the seconds it runs from and to, the kind of shot, a description,
  and a box per prompt block.
- **Add scene** at the bottom, numbered after the last.

Every field saves when you leave it. A time is typed as `1:23` or `0:04.5`
and stored as seconds; a scene cannot end before it starts. The kind of
shot is chosen from the profile's list, never typed: a scene called
*closeup* when the list says *Close-up* would never be found by the strip.

## Prompt blocks

Each block has its own **copy** button, because a block goes into a
generator as it is — the still-frame prompt into the picture model, the
animation prompt into the video model, the negative beside either. The
blocks a scene carries are the kind's `scene_blocks`; a block the profile
no longer names is still shown, read-only, so no text is ever hidden.

The blocks are written by hand, or proposed by an agent outside the window:
`propose_scenes` over [MCP](/kilna/reference/mcp/#a-video-from-a-song-end-to-end)
lands a whole board in the chat on the work, every block filled, with
**Add to the board** or **Replace the board** under it. Actions on the
Scenes tab that do the same from inside kilna come in v0.63.

## What else a scene touches

- **Undo** takes back adding a scene, an edit to one, and its deletion.
  `Ctrl+Z` works from anywhere, and the toast after each offers the same.
- **The trash** keeps a deleted scene under its work — *Scene 4 · chorus*
  — and takes a work's scenes with the work, bringing them back with it.
- **The history** says *Scene 4 added to "Harbour lights"*, and when one
  is deleted.
- **The export** writes a *Scenes* section on the work's page: the board
  as a table, then each scene's blocks.
- An agent over [MCP](/kilna/reference/mcp/) sees the kinds of shot
  and blocks in `workspace`, how many scenes a work has in `work`, and the
  whole board with `scenes`; it proposes a board with `propose_scenes`, or
  a new video with its board in one `propose_work` package. A replaced
  board rewrites a scene with the same number in place — it keeps its id —
  and sends the rest of the old board to the trash, one operation per
  scene, so undo walks it back the way it walks back your own edits.

Why a scene is a row rather than a chapter of a text, and why it belongs to
the work, is in
[ADR 0020](https://github.com/lacodda/kilna/blob/main/docs/adr/0020-a-scene-is-a-row-owned-by-the-work.md).
