---
title: The interface
description: How kilna's window is laid out — the sidebar, the topbar, themes, and URLs you can navigate by.
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

## The loop underneath

The frame serves the same five steps everywhere: a work gains versions, a
version earns a score, a score wins a calendar slot, and the slot ends in a
release you mark by hand. See [The loop](/kilna/concepts/the-loop/) for why
each step exists.
