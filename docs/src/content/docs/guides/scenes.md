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
- **AI actions** — the profile's actions for this kind: in Studio, the
  plot from the source and the storyboard from the plot. A click starts
  one; what comes back is a proposal, applied with one button. See
  [Actions on the board](#actions-on-the-board).
- **A strip of kinds of shot** with counts — *All · Wide · Close-up ·
  Detail…* Click one to see only those scenes: "show me every detail" is
  one click, and the chip that is on turns off.
- **The board as a table**, one row per scene: the number, the section
  (*intro*, *verse 1*, *chorus*), the seconds it runs from and to, the kind
  of shot, the first line of the description, how far the scene is filled
  in, and the profile's scene actions as icons — *Prompts for the scene* in
  Studio. Fifty scenes read as fifty rows you can compare, not fifty cards
  you scroll past.

  **Click the description to open the row.** Underneath it, the description
  in full and a box per prompt block, each with its own copy button. The
  long text is folded away, not left out.

  **Filled in** is computed, never stored: *empty* when nothing is written,
  *ready* when the scene has a description and every prompt block the
  profile names, *started* in between. A kind with no prompt blocks asks
  only for a description.
- **Add scene** at the bottom, numbered after the last, and — once the
  board has scenes — **Time the board**.

Every field saves when you leave it. A time is typed as `1:23` or `0:04.5`
and stored as seconds; a scene cannot end before it starts.

### Who is in it, where it happens

A scene points at the notes it is about — a character, a place — rather
than describing them again in the shot. A hero written three ways across
fifty descriptions is three heroes to anything that reads them; a
reference is one person.

Open a row and choose from the notes whose kind the profile names as a
kind of note (`character`, `location`, `lore` in Studio — see
[the profile document](/kilna/reference/profile-document/#note_kinds)).
The names show in the **About** column, and a chip appears above the
board for everyone the board names: click one and the board narrows to
**every scene with her in it**.

A character is a note, not a table of its own: its own editing screen
comes later and edits that same note. Deleting a scene takes its
references with it and leaves the notes alone; deleting a character takes
her out of the scenes, and restoring her from the trash puts her back
into them.

### Framing the board from the text

An empty board offers to build itself from the source. A video is made from
a song ([Links](/kilna/guides/made-from/)), and a lyric names its own parts —
`[Verse 1]`, `[Chorus]`, `(Bridge)`, in any language. **Frame the board
from:** and the role's name gives you one scene per part, in the text's own
order, each carrying the part's name as its section.

The description is left empty on purpose: what is *seen* in a scene is not
what is *sung* in it, and lines pasted into the description would be words
you have to delete before writing the shot.

Only a line that is *nothing but* a marker counts — a bracket inside a line
belongs to the line, and a `## Heading` is not a marker at all. A part named
with nothing under it (`[Instrumental]`, `[End]`, a `[Chorus]` repeated by
marker alone) is still a part, and becomes a scene like any other.

A board that already has scenes is left alone; empty it first if you want a
new frame. A text with no markup says so rather than producing one scene
holding the whole song. Framing is one change, so one **Undo** puts the
whole frame in the trash.

### Timing the board

**Time the board** divides the work's `duration` — a number of seconds in
its overview fields — evenly between the scenes, in the order they stand.
It is the board's first timing rather than its last word: drag the spans
by hand from there, and the neighbours are yours to settle.

The spans join, and the last scene ends on the length itself, so the board
covers the work with no gap and no overhang. A work with no duration is
refused rather than timed: fifty scenes all starting at zero is worse than
fifty untimed ones. Timing is one change, so one **Undo** puts every span
back the way it was — including a span you had set by hand. The kind of
shot is chosen from the profile's list, never typed: a scene called
*closeup* when the list says *Close-up* would never be found by the strip.

## Prompt blocks

Each block has its own **copy** button, because a block goes into a
generator as it is — the still-frame prompt into the picture model, the
animation prompt into the video model, the negative beside either. The
blocks a scene carries are the kind's `scene_blocks`; a block the profile
no longer names is still shown, read-only, so no text is ever hidden.

The blocks are written by hand, or proposed: by an action on the row (below),
or by an agent outside the window — `propose_scenes` over
[MCP](/kilna/reference/mcp/#a-video-from-a-song-end-to-end) lands a whole
board in the chat on the work, every block filled, with **Add to the board**,
**Replace the board** or **Revise the scenes** under it.

## Actions on the board

The profile's actions for a kind with a storyboard sit above the board, and
a scene action sits on each row. Studio ships three, and a profile can
carry any number — see
[`prompts`](/kilna/reference/profile-document/#prompts):

1. **Plot from the source.** The video is [made from](/kilna/guides/made-from/)
   a song; the action reads the song's lyrics and writes the plot, kept as
   a version in the `plot` role — the video's story beat by beat against
   the sections of the text, with the recurring things named the same way
   every time. A video made from nothing is told to link its source first.
2. **Storyboard from the plot.** Reads the plot, the context and the board
   as it stands, and proposes the whole board: one scene per beat, the
   kinds of shot varied on purpose, a description that says what is seen.
   No prompt blocks — those are written per scene. **Replace the board**
   rewrites a scene with the same number in place, so what pointed at it
   still does; the plot and the context have to be written first, and the
   action says so if they are not.
3. **Prompts for the scene.** On the row. Reads the context, the board for
   continuity and this scene, and proposes its prompt blocks — the still,
   the animation, the negative — as a **revision**: only this scene, only
   the blocks. **Revise the scenes** writes them; the rest of the board is
   not touched, and nothing goes to the trash.

The eye beside each button shows exactly what it sends — the message with
the board filled in, the method behind it — and takes reference files by
path: a picture of the hero, a frame from the last video. An empty board
is an entrance to these actions rather than an empty tab.

The action's answer is read against the profile: a block in the wrong
words — a kind of shot the profile does not have, a revision that numbers
another scene — is not silently nothing; the chat says why there is no
button, and the answer is still there to read.

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
