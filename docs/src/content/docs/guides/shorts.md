---
title: Shorts
description: Which stretches of a video a short is cut from, the prompt its cover is drawn from, and how the calendar keeps two shorts of one video apart.
---

A short is cut out of a finished video, or shot for itself. In kilna it is
the same kind of work either way — the difference is whether it has a
**donor**, and if it does, which parts of it.

## Saying what it was cut from

Name the video on the short's [Links](/kilna/guides/made-from/) tab, or make
the short from the video with *Make a Short from this*. Either way the short
now has a donor, and a **Cut** tab appears beside its Scenes.

The Cut tab draws the donor as a track of its own length, with each stretch
you have taken sitting where it actually falls in it. That is the point of a
track rather than two boxes of numbers: deciding whether to take another
twelve seconds is a question about *where*, relative to what you already
took, and two numbers cannot be looked at that way.

**Take from** adds a stretch, starting where the last one of that video
ended — taking three in a row is three clicks rather than three sums. Under
the track each stretch shows its two ends as timecodes you can type into:
`48`, `0:48`, `1:23.5` all read. The eye places a cut and the keyboard
finishes it; nobody drags to exactly 48.0.

A short spliced from two different videos draws two tracks. Drag the rows to
change the order they are spliced in — the order is the splice, and the
number on the left is where each stretch lands in the finished thing.

The track needs the donor's length to draw against. A video whose
**duration** is not filled in yet says so instead of drawing: a track with no
scale would place every cut at an arbitrary point and look exactly as
convincing as a real one. Set the duration on the donor's Overview tab.

## The cut list

At the bottom of the tab is the **cut list**: the stretches in order, each
beside the donor's video file. It says one of three things.

- *Nothing to cut yet* — the splice is empty.
- *A video this is cut from has no file attached yet* — attach the rendered
  video to the donor on its [Files](/kilna/guides/files-and-covers/) tab.
- *Every stretch has its file* — and **Save the cut list** writes it out as
  JSON.

kilna does not cut the file. It keeps the boundaries and hands them out; the
cutting is a [plugin's](/kilna/guides/writing-a-plugin/) job, and until one
is installed the saved list is a file you can point anything at. The plugin
reads exactly the same list.

## The cover's prompt

A short is picked off a wall of thumbnails, so its cover is written and
reworked the way the thing itself is. The **Files** tab of a kind whose
profile names cover parts opens with them — in the shipped music profile a
video and a short get three:

| Part | What goes in it |
| --- | --- |
| Picture | What the thumbnail shows: subject, framing, light, mood. |
| Negative | What must not appear on it. |
| Typography | The words on the cover, and how they sit. |

Each is copied on its own, because each goes into a different field of
whatever draws it. The picture that comes back is attached below, on the same
tab, as the work's cover — writing the prompt and looking at the result are
one activity and belong on one screen.

These are your craft's words, not kilna's: rename them, drop one, add a
fourth in the [profile document](/kilna/reference/profile-document/) under
`cover_blocks`. A kind that names none — a song whose cover is the album's —
draws nothing here.

The prompt is searched with the rest of the work, so the short you remember
by its picture is findable by the words that made it.

## On the calendar

Two shorts cut from one video never land on neighbouring days. However they
are titled, to anyone watching they are the same video twice, so the
[layout](/kilna/guides/planning-a-release/) treats them the way it already
treats two releases of one work: it keeps them apart and fills the day
between with something else.

A short shot for itself has no source to clash on and is free to take that
day. There is no separate shorts queue and no shuffle — one rule, applied
every time, on the video each short is actually made of.

## What else a splice touches

- The **donor's** card lists what has been taken out of it, at which
  moments.
- The **history** says *A stretch of "Harbour lights" taken into "The hook"*.
- **Undo** takes back a stretch, a moved end, or a reordered splice.
- The **trash** keeps stretches with the work it takes down, from either
  side: restore the short and its splice is back, restore the video and the
  marks the shorts made in it are back too.

See [ADR 0029](https://github.com/lacodda/kilna/blob/main/docs/adr/0029-a-short-is-a-work-with-a-donor-and-a-list-of-stretches.md)
for why the boundaries are a list rather than a pair of numbers.
