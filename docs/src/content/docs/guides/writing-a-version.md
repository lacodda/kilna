---
title: Writing a version
description: The editor — drafts that survive a closed window, naming versions, previewing markdown, writing full screen, and comparing two drafts side by side.
---

Every draft of a work is kept whole. You never overwrite the last one, and
nothing is stored as a diff — so going back to what you wrote three weeks ago
is opening it, not reconstructing it.

This is the **Versions** tab of a work's card.

## Your draft is not lost

Text in the editor is kept as you type, and it is still there after you close
kilna and open it again. A draft belongs to one work and one role: what you
write as lyrics stays under lyrics, and switching to the style prompt gives you
that role's own draft rather than carrying yours over.

A small *Draft kept* note appears while there is unsaved text. It is a
reassurance, not a state you have to clear — the draft disappears by becoming a
version, and in no other way except your own deletion. Emptying a work from the
trash for good takes its drafts with it, since nothing could reach them
afterwards.

## Saving a version

**Save as new version** turns the draft into a version with the next revision
number for that role. Two things are worth setting before you do:

**A name.** Optional, and worth it. `tightened chorus` says more in the list a
month later than `Revision 4`. Unnamed versions are listed by their number, so
there is no penalty for skipping it.

**Whether it becomes current.** Ticked by default, because a new draft is
usually the one you are working on. Untick it to record an experiment without
promoting it — the work keeps pointing at the version it pointed at before, and
scores and exports keep reading that one. You can always promote it later with
the star beside its row.

## Reading what you wrote

The open version can be read two ways:

- **Text** shows exactly what is stored, in a monospace column.
- **Preview** renders it as markdown — headings, emphasis, lists, quotes.

The same two modes are in the editor while you write, so you can check how a
lyric sheet or an outline will read before it becomes a version.

kilna stores your text exactly as you typed it. Markdown is a way of *looking*
at it, never a conversion: nothing is rewritten, and a draft full of asterisks
you did not mean as emphasis is still stored with those asterisks.

## Full screen

The **full screen** button takes the editor over the whole window. A page of
lyrics read inside a six-row box is not read at all. `Esc` leaves it, and
nothing about the draft changes on the way in or out.

## Comparing two versions

The **±** button on any version in the list compares it with the one currently
open. They appear side by side: the older text on the left, the newer on the
right, with lines that left marked in red and lines that arrived marked in
green. A line at the top says how much moved.

The comparison is by line rather than by word, because a version here is
prose — a verse, a scene, a script — and prose is revised by the line. Seeing
*this line went, that one arrived* answers what changed between two drafts;
a word-level diff of a rewritten verse is confetti.

Press **±** again to stop comparing, or open the version you were comparing
against and it clears itself.

## Roles

A work's versions are grouped by **role**, and the roles come from your
[profile](/kilna/concepts/profiles/): a song has `lyrics` and `style`, a chapter
has `text`, `outline` and `notes`. Roles advance independently — lyrics v4 has
nothing to do with style v2 — so the tab shows one role at a time rather than
interleaving them.
