---
title: Files and covers
description: Attaching pictures to a work — a cover that shows everywhere, references kept beside it, and why a file is copied into the workspace rather than pointed at.
---

A work can carry files: a cover, a reference, a picture of the thing. They
live on the card's **Files** tab.

## Attaching a file

Two ways in. **Attach a file** opens the picker; **drop files onto the tab**
does the same for as many as you drag at once. Either way kilna takes
pictures — PNG, JPEG, WebP, GIF, AVIF.

**Set a cover** attaches a file and marks it as the work's cover in one
step. Setting a second one changes the cover: the newest is the one that
shows, and the one it replaced stays attached rather than disappearing
behind your back. Remove it and the older one is the cover again.

Where the cover is drawn rather than photographed, it is built on the card's
**Cover** tab - see [The cover](/kilna/guides/the-cover/). The pictures a
generator gives back land there as **candidates**, and one is made the cover;
here they are a group of files of their own.

## The cover's shape

The place a release goes out decides the shape of its picture: YouTube wants a
16:9 preview, Shorts a 9:16 frame, a streaming service a square. The shape is
the kind of release's (`cover_format` in the
[profile](/kilna/reference/profile-document/)), so a work that goes out in two
places needs two covers. The **Cover** tab of a clip, an audio release or a
short offers the shapes of the doors its kind has, and says of a shape it
does not go out through yet that it does not.

## The frame

An audio release on a video platform is a track with a picture on it: one
still that stays on screen for the whole song, and a short loop of what moves
in it - a leaf drifting across, dust turning in a beam. A kind whose works
play under one (`"frame": true`; Studio's **Audio**) has a **Frame** tab:

- **the still** - what the picture shows, as you would describe it to a
  generator;
- **what moves** - the one thing alive in a picture that otherwise holds
  still;
- **the loop's length** - 4, 6, 8 or 10 seconds;
- **the camera does not move** and **seamless: the last frame matches the
  first** - two switches, on by default;
- **the negative** - what must not appear.

Since v0.88 the still is **built from the cover** by default - the same idea,
hero, style, ground and mark, without the title - and what you write is added
to it as your own words; **own scene** keeps the still as you write it,
whole. See [The cover](/kilna/guides/the-cover/#the-frame-of-an-audio-release).

Beside them, three blocks to copy into the generator - the still, the loop and
the negative. The loop's block is written for you from the settings - *LOOP
(6 s): the beam turns. The camera does not move at all. Seamless loop: …* -
because a still camera and a seamless join are instructions a model reads,
not a mood it guesses from the picture. Each setting says its sentence and
nothing else does. See
[ADR 0046](https://github.com/lacodda/kilna/blob/main/docs/adr/0046-a-frame-is-a-still-and-a-loop-written-from-its-settings.md).

## Where the file goes

**A file is copied into the workspace**, not pointed at where it lay. The
copy sits in a `media` directory beside the workspace database, named by an
id of its own.

This is the difference between a cover that keeps working and one that
breaks the day you tidy your downloads folder. It also means the pictures
travel: **a backup takes the files with the database**, into a directory
named after the backup, and restoring brings both back. A workspace is one
folder, and moving it is moving a folder.

The name the file arrived under is kept and shown — it is what you
recognise it by, and often what a generator wrote in it. Inside the
workspace the file is named by its id, so two files called `cover.png`
cannot collide.

Removing a file takes the copy with it. The original, wherever it came from,
is never touched. This is not the trash: what the trash promises is that a
deletion can be taken back, and a row restored beside bytes that are gone
would be that promise broken.

## Where a cover shows

Everywhere a work is drawn small:

- the banner at the top of its card,
- beside its title in the catalogue,
- on its chip in the calendar,
- on the dashboard and in the search palette.

A work with no cover keeps the colour it has always had — a gradient derived
from its id, so a row of works reads as one family. The picture is laid
*over* that colour rather than replacing it, so a cover still loading, or
one whose file went missing, shows the work's own colour instead of a white
hole.

## Files in groups

The tab shows a work's files grouped by what they are for: the cover first,
then the candidates for it, then the stills its scenes are drawn from, then the clips animated from
those, then everything else. A group with nothing in it is not drawn at all.

The grouping earns its place on a storyboard. Scene stills and clips are
files on the work like any other, so a board of fifty scenes with four
candidates each puts two hundred pictures in this tab — in one flat list the
cover would be somewhere in the middle of them. Scene material is normally
handled on the board itself (see [Scenes](/kilna/guides/scenes/)); this tab
is where you see all of it at once, and what it costs.

A file may belong to more than one work: copying a board points the copy at
the same pictures rather than duplicating them. Removing it from one work
leaves the other looking at it, and the bytes go only when the last work
using them lets go.
