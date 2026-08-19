---
title: Finding things
description: One box that searches works, drafts, notes and assistant replies — what it looks in, how it ranks, and why it works in Russian too.
---

`Ctrl+K` from anywhere opens the search box. So does the field in the top bar,
if your hands are on the mouse.

It looks in four places at once, and groups what it finds:

| Group | What is searched |
| --- | --- |
| Works | Titles |
| Drafts | The full text of every version, in every role |
| Notes | The body and the title of notes attached to a work |
| Assistant | What was said in chats about a work |

Each hit opens the work it belongs to, on the tab where the hit lives — a line
found in a draft opens Versions, a note opens Notes.

## It is for recognising, not for browsing

The box shows a few hits per group rather than everything that matched. Past
half a dozen the list stops being scannable, and the useful answer is a
narrower query rather than a longer list.

That is also why hits are shown as text rather than as counts: a draft hit is
the line it was found in, with a little of what surrounds it, so you can tell
*this* verse from *that* one without opening either.

## The keyboard is the point

- `Ctrl+K` opens it, from anywhere — including from inside a text field.
- `↑` and `↓` walk the hits, across group boundaries.
- `↵` opens the highlighted one.
- `Esc` closes without going anywhere.

## Case does not matter, in any language

Searching for `гавань` finds **Гавань огней**, and `HARBOUR` finds
*Harbour lights*.

That is worth saying because it is not free. SQLite's own case-insensitive
matching covers ASCII and nothing else — to it, `Г` and `г` are unrelated
bytes. kilna folds case itself, in full Unicode, which is why a Russian
workspace is as searchable as an English one.

The same fix applies to the search field above the works list, which had the
same blind spot.

## What it does not do yet

- **No filters or operators.** No `kind:song`, no quoted phrases. One box, one
  query, everything at once.
- **Whole words only in the sense that substrings match** — `arbour` finds
  *Harbour*. There is no stemming, so `lights` does not find *light*.
- **Nothing outside the active profile.** Switching profiles switches what is
  searchable, like everywhere else in kilna.

Search reads every body in the profile on each query. That is instant for a
workspace of a few hundred works and will not stay instant forever; when it
stops being instant, the answer is an index, and the place to put one is the
same function this box already calls.
