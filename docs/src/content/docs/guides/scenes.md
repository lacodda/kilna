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
  in full and a box per prompt block, each with its own copy button — and a
  **regenerate** button that aims a scene action at that block alone, with
  the eye beside it showing exactly what would be sent. Two blocks of a
  scene regenerate side by side; the same block twice does not start
  again while it is going. What comes back is laid over the scene, so a
  rewritten animation cannot delete the still. The
  long text is folded away, not left out.

  **Filled in** is computed, never stored: *empty* when nothing is written,
  *ready* when the scene has a description and every prompt block the
  profile names, *started* in between, and *shot* once one of its
  [frames](#frames) is chosen. A kind with no prompt blocks asks only for a
  description.
- **Add scene** at the bottom, numbered after the last, and — once the
  board has scenes — **Time the board**.

Above the board sits [what it still owes](#what-the-board-still-owes): the
counts, and the list of what is missing.

Every field saves when you leave it. A time is typed as `1:23` or `0:04.5`
and stored as seconds; a scene cannot end before it starts. The **number**
is a field too — see [Numbering the board](#numbering-the-board).

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

## Frames

A prompt goes into a generator and pictures come back — usually four for one
prompt, because choosing between them is the work. All four belong on the
scene: open its row and they sit in a strip under the prompt blocks.

A picture gets there three ways, whichever is nearest:

- **Add a picture** opens the file picker.
- **Drag a file onto the window** while the scene's row is open.
- **Ctrl+V** pastes a picture straight from the generator's own tab — no
  saving to disk first.

The file is copied into the workspace the same way any other is (see
[Files and covers](/kilna/guides/files-and-covers/)), so a frame travels with
a backup of the workspace and keeps working when the original is tidied
away.

**Choosing one.** The tick on a frame says the video is cut from that one.
A scene has at most one chosen frame — choosing a second moves the mark
rather than adding it — and **Choose none** goes back to having candidates
and no verdict, which is an ordinary place to be in the middle of the work.
The chosen frame appears beside the scene's number on the board, so the
storyboard answers both questions at a glance: which picture is this scene,
and which scene is this picture.

**Looking at them properly.** Click a frame to open it full screen. The
arrows — and the ← and → keys — walk from scene to scene rather than between
the candidates of one scene: at that size what you are checking is whether
the story reads. Scenes with no frame yet are skipped. Escape closes.

**Removing one** takes the picture out of the workspace with it. Like
detaching any file, it is not undoable: a row put back beside bytes that are
gone is a broken picture, not an undo. A picture shared with a copy of the
board stays on disk until the last board using it lets go.

## Video

Under the pictures is a second strip, for the clips animated from them. It
works the way the frames do — the picker or a file dragged onto the window,
several takes side by side, a tick on the one the montage uses — with two
differences.

`Ctrl+V` is not there: a clip is downloaded and then dragged, not copied out
of a browser tab the way a picture is.

And the two verdicts are independent. A scene has at most one chosen still
**and** at most one chosen clip, so picking a take never disturbs the picture
it was animated from, and **Choose none** on one says nothing about the
other. A scene counts as *shot* when its **picture** is chosen — a clip
picked while the still never was is not a finished scene, it is a scene in a
state nobody meant.

## The montage list

**Copy the montage list** puts the whole board on the clipboard as text;
**Save the montage list** writes the same thing to a `.txt` you name. A
scene takes one block — its number and span, then the files it uses:

```
01  0:00–0:04  C:\Users\you\kilna\media\a1b2c3.png
               C:\Users\you\kilna\media\d4e5f6.mp4
02  0:04–0:09  C:\Users\you\kilna\media\7a8b9c.png
               —
```

It is text rather than a project file on purpose: an `.edl` or an `.xml` is a
better answer for exactly one editing program and a worse one for every
other, and text pastes into any of them, into a note, or into a terminal
unchanged.

Two things it does deliberately. It names only what was **chosen**, not every
candidate — otherwise it would be a list nobody could hand to an editor. And
it is always the whole board, never the filtered view: a cut is the video end
to end, and a list quietly missing the scenes a filter hid would be worse
than no list. A scene with nothing chosen still gets its line, with a dash,
because the list is also how you see what is still missing.

## The whole thing in a folder

The montage list is what one editing program needs. **Export a package** is
what a person needs: you choose a folder, and kilna makes one inside it named
after the work, holding everything at once.

```
The long way round-9f3a1c2b/
  board.md       the board as a table, then every scene with its prompts in full
  release.md     what each release goes out as: title, description, tags, comment
  material/
    01-still-chosen-harbour-v3.png
    01-clip-1-chosen-harbour.mp4
    02-still-2.png
```

The pictures are copied under names that say what they are — the scene number
first, so the folder sorts into the order the video is cut in, then whether it
is a still or a clip, then whether it is the chosen one. A folder of files
named by their ids is a folder you open one by one.

Everything in it is text and ordinary files. Nothing in the folder needs kilna
to read, which is the point: the folder is what you have in front of you while
the thing is actually made.

`release.md` is left out entirely when nothing has been written about any
release yet, rather than written as an empty heading — see [what a release
goes out as](/kilna/guides/planning-a-release/#what-a-release-goes-out-as). A
picture whose file has gone missing does not take the package with it; the
rest is still written, and `board.md` still says which scene it belonged to.
Afterwards kilna says how many scenes still have nothing chosen, because a
package is also how you find out what the board is missing.

## A second attempt

**Make a second attempt** copies the board into a new video and leaves this
one exactly as it is — the two are meant to be compared.

The copy hangs on the same donor, so it can still be reframed from the same
text, and it brings every candidate across rather than only the chosen ones:
the pictures you are still deciding against are part of the work. The
verdicts come with them, so a board that had chosen its stills arrives having
chosen them.

What does not come across is the score and the releases. A copy has not been
judged and has not gone out, and carrying that over would be the copy
claiming the original's reception.

The pictures themselves are not duplicated on disk — both boards point at the
same files, which is why a clone of fifty scenes is instant and costs
nothing. Removing a picture from one board leaves the other looking at it.

`Ctrl+Z` takes the whole clone back, board and all.

## Numbering the board

A scene's number is its place in the cut, so the number is where the board
is reordered from. **Type a number on the row** and the scene goes there;
the rest close up behind it.

- Scene 12 said to happen third becomes scene 3, and what was 3 to 11
  each move down one.
- **Add a scene after this one** — the icon on the row — puts a new row
  directly below instead of at the end of the board.

Either way the board comes out numbered 1, 2, 3… with no hole and no two
scenes sharing a number, whatever it looked like before. A number outside
the board is refused rather than guessed at, and the field goes back to
what the scene actually holds.

**The pictures follow their scenes.** Nothing is renamed and no file
moves: a frame belongs to a scene, not to a number, and a file in the
workspace is named by its own id. Scene 12's four candidates are scene 3's
four candidates a moment later, with the chosen one still chosen.

**The seconds stay where they are.** Reordering says what happens in what
order, not when; the spans are yours to settle afterwards, by hand or by
timing the board again. A span that travelled with its scene would put the
third scene at 0:48 because it used to be the twelfth.

`Ctrl+Z` takes a renumbering back whole — the board returns to the order it
stood in, not scene by scene.

## What the board still owes

A storyboard looks finished long before it is. Fifty rows all carrying a
number and a description read as a full board at a glance; the four
without a prompt, the hole between scene 11 and 12, and the minute of
video nothing accounts for turn up later — in the editing program, with
the files already in hand.

The panel above the board is that pass, made before then. It counts what
is done — *50 scenes · 47 written · 31 with a picture · 8 with a clip* —
and lists what is missing:

- **A scene nobody has touched**, and a scene written only halfway.
- **A scene with no picture drawn for it**, kept apart from one with
  pictures and no verdict: the first needs a generator, the second needs
  you. Clips are told apart the same way.
- **A hole or an overlap** between two scenes, with how big it is.
- **Scenes with no seconds**, counted once rather than listed, while the
  board is only partly timed.
- **The board and the work's length** parting company.

**A stretch saying the same thing is one line.** A board moves through its
stages in one piece: everything is written before anything is drawn. So
forty scenes all waiting for a picture read as *Scenes 3 to 42 are waiting
for a picture — 40 of them*, not as forty copies of one sentence. Two stay
two — two lines are two scenes — and scenes that are **not** neighbours
never join: three holes in work that is otherwise finished are three places
to go, and that is exactly what the list is for.

**Click a line to go to the scene** — the row opens where you land, and a
stretch takes you to the first of it, where the work resumes.

Nothing here is stored or has to be asked for: the counts are read off the
board as it stands this second, by the same rule the *Filled in* column
uses, so the panel and the rows cannot tell you two different things. A
board where nothing is timed yet is not nagged about timing; that is a
board at an earlier stage of the work, not a board with fifty faults.

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

- **Undo** takes back adding a scene, an edit to one, its deletion, a
  renumbering of the whole board, and a second attempt at a whole video.
  `Ctrl+Z` works from anywhere, and the toast after each offers the same.
- **The trash** keeps a deleted scene under its work — *Scene 4 · chorus*
  — and takes a work's scenes with the work, bringing them back with it.
- **The history** says *Scene 4 added to "Harbour lights"*, when one is
  deleted, when the board is renumbered, and when a video is started as a
  second attempt.
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
