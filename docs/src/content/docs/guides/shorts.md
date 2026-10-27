---
title: Shorts
description: Which stretches of a video a short is cut from, the prompt its cover is drawn from, and how the calendar keeps two shorts of one video apart.
---

A short is cut out of a finished video, or shot for itself. In kilna it is
the same kind of work either way — the difference is whether it has a
**donor**, and if it does, which parts of it. It is a work, not a door on
the song: since v0.74 a song ships only as an audio release, and a short
ships through its own door, on its own card, with its own date.

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
beside the donor's [finished file](/kilna/guides/files-and-covers/#the-finished-file) -
the clip in the video's folder on disk, found by its kind's rule or chosen on
its Files tab. It says what stands in the way, or that nothing does:

- *Nothing to cut yet* — the splice is empty.
- *No folder on disk to find its file in* — set the media folder in
  **Settings → Data**, or name the folder of the video's kind.
- *No finished file yet* — nothing in the video's folder matches its kind's
  rule: the clip is not rendered, or lies somewhere the rule does not look.
- *Its kind names no rule* — choose the clip on the video's Files tab.
- *The file chosen for it is gone* — choose it again.
- *Every stretch has its file* — and **Save the cut list** writes it out as
  JSON.

kilna does not cut the file. It keeps the boundaries and hands them out; the
cutting is a [plugin's](/kilna/guides/writing-a-plugin/) job. A cutting
plugin's commands stand right here, under the list: it is handed the list, the
short's folder, the song's name and the short's cover in one call, shows how
far it has got while it runs, and can be stopped -
[Cutting a short](/kilna/guides/cutting-a-short/) installs one and cuts with
it. Without one, the saved list is a file you can point anything at - the
plugin reads exactly the same list.

When the short has a finished file of its own - one a plugin made, or one
rendered elsewhere into its folder - the list names it, with how long it runs
beside how long its stretches add up to: the one check a cut needs before it
goes out. The length is read from the file by the window's player; kilna
itself never opens the video.

## The cover's prompt

A short is picked off a wall of thumbnails, so its cover is built and reworked
the way the thing itself is - on its **Cover** tab, in the vertical 9:16 of
the Shorts door. The tab builds the prompt from parts: the idea and the
scene, the hero, a built frame, the styles, the channel's mark and details;
the picture that comes back is dropped onto the same tab and made the cover.
See [The cover](/kilna/guides/the-cover/).

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
