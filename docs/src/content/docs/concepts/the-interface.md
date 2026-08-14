---
title: The interface
description: How kilna's window is laid out — the sidebar, the topbar, themes, URLs you can navigate by, and how the app tells you what it did.
---

kilna's window is a frame around one working surface. The frame does not
change as you move: a sidebar on the left, a slim topbar above, and the
current screen filling the rest.

## The sidebar

The sidebar lists the screens: **Works** (the list and the open work),
**Catalogue** (every work with its score and tier), **Calendar** (the queue
and the taken slots), and **Settings** (data in and out, the profile editor).

Some entries are doors that are not built yet — Dashboard, Collections,
Notes, Trash. They sit in the sidebar with a small version chip naming the
release that delivers them, so the map of what is coming lives in the app
itself rather than in a changelog.

The footer holds the theme switch, the profile picker, and Settings.

## Themes

kilna is a studio tool, so it is dark by default — more precisely, it
follows your system's theme until you say otherwise. The theme button cycles
through *system*, *light* and *dark*; the choice is remembered across
launches and applies before the first paint, so the window never flashes the
wrong color.

## Screens are URLs

Every screen has an address, and the open work is part of it: `/works/abc123`
is that work, opened. The back button walks your actual history, and anything
that can hold a link — a note, a chat message, a journal entry — can point
at a screen or a work directly.

## When something happens

kilna tells you what it did, and stays out of the way when there is nothing
to say.

**Saving.** Most fields save when you leave them — there is no Save button
to hunt for. A small *Saving… / Saved* marker appears next to the field
while that is happening, then fades. If a save fails, the field goes back to
what it held before, and a message explains why: nothing is left looking
saved when it is not.

**Messages.** Completed actions announce themselves briefly in the bottom
right — a work added, a slot claimed, something deleted. They disappear on
their own. A message never asks you a question; anything that needs an
answer is a dialog you can cancel.

**When something goes wrong.** Failures are written as sentences about your
work, not as the database's own words. "That is not here any more — it was
probably deleted in another view" is the whole message; the technical detail
stays out of your way unless a screen crashes outright, and then it is
folded behind *Technical detail* for a bug report.

**When a screen breaks.** A screen that stops working is replaced by a small
panel with a *Try again* button. The rest of the window keeps running, and
moving to another screen clears it — one broken screen never takes the app
with it.

**While loading.** Lists and cards show a grey outline of what is arriving
rather than an empty box or a spinner. A screen with nothing in it yet says
what it is for and offers the one thing worth doing there.

## The loop underneath

The frame serves the same five steps everywhere: a work gains versions, a
version earns a score, a score wins a calendar slot, and the slot ends in a
release you mark by hand. See [The loop](/kilna/concepts/the-loop/) for why
each step exists.
