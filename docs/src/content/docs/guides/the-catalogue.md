---
title: The catalogue
description: The one list of everything you are making — sorting, filters that find unfinished work, and acting on several works at once.
---

The catalogue is every work there is. Add one here, find one here, open one
here. There is no second list.

Each row carries the work's title, its status and kind, the tier and score it
last earned, and when it was scored. Clicking a row opens it.

## Choosing the columns

The **columns button** above the table, to the right, says which columns it
draws. Beside the ones it opens with there are four more:

| Column | What it says |
| --- | --- |
| ID | The work's identifier, shortened — what a plugin or the assistant calls it |
| Marks | The flags your profile offers, raised on this work by hand |
| Versions | How many versions the work holds, across every role |
| Added | When it was first written down |

The title cannot be turned off: it carries the row's identity and the link into
the work.

**Your choice survives a restart**, like the sort and unlike the filters. It is
how you prefer to read the table, not what you are doing this minute. It is kept
per machine rather than per profile, so switching craft does not change the
columns — that moves onto the profile when the profile document next changes
shape.

A mark your profile no longer defines is not drawn, the same rule the card
follows: renaming a mark never touches a work.

## Grouping

The **group** dropdown folds the table into blocks by status or by tier. Click a
block's heading to collapse it; the count beside the heading says how many rows
are inside.

The sort still decides the order, so a catalogue grouped by tier and sorted by
score opens on the tier holding your best work. Works with no value — no tier
yet — gather in a block of their own at the bottom, for the same reason unscored
works sort last.

**Grouping is forgotten when you close kilna**, like the filters. It is a way of
interrogating the list today.

Grouping by collection is deliberately not offered yet: collections have no
screen of their own until a later version, and grouping by something you cannot
open or edit would promise the feature twice.

## Sorting

Click a column heading to sort by it; click it again to turn it around. An
arrow shows which column is doing the sorting and which way.

Every column sorts except the two that answer no ordering question — the ID and
the marks. Numbers and dates open at the top: the best score, the most recent
edit. Words open at A.

**Unscored works stay at the bottom either way.** They are not the worst work
you have, they are the work nothing has judged yet, and floating them to the
top of a "worst first" sort would bury the answer you asked for.

The sort is remembered between sessions — it is how you prefer to read the
table, a standing choice.

**Filters hold for as long as kilna is open, and no longer.** Opening a work
and coming back is the commonest thing anyone does here, and a filter that did
not survive it made the catalogue tiresome to use. Keeping filters across
restarts would be the opposite mistake: finding the catalogue still hiding most
of your work the next morning reads as lost data rather than as a setting you
left on. Closing the window is the reset, and it is one you never have to
remember to perform.

## Narrowing the list

The search box matches titles as you type, in whatever alphabet they are
written in. The three dropdowns narrow by status, kind and tier.

**The chips below them find unfinished work** — the gaps you can still do
something about:

| Chip | What it finds |
| --- | --- |
| Not scored | Nothing has judged these yet |
| Nothing planned | Judged, but nothing has gone out and nothing is booked |
| Score is old | The work changed after it was scored |

*Nothing planned* deliberately leaves out anything already released: it needs
nothing from you. That rule comes from the predecessor of this app, whose
first version of the same idea listed ten songs that had already shipped and
offered to fix them.

One chip at a time; clicking the one that is already on turns it off.

While anything is narrowing the list, a count says how much you are not
seeing — *1 of 12* — with **Clear** beside it. When nothing is filtered the
count stays out of the way, because "12 of 12" tells you nothing.

## Working on several at once

The checkbox on each row picks it out; the one in the heading takes every row
currently shown — filtered-out works are never caught by it.

**Shift-click takes everything in between.** Tick one row, then shift-click
another, and the run between them follows the first row's state: dragging out
of a ticked row fills the span, dragging out of an unticked one clears it. The
run is what you see, in the order the table is showing it.

A bar appears above the table with what you can do to the chosen rows:

- **Move to status** — sends the whole selection to one status. Chosen by hand,
  so each one holds there: the automation leaves a hand-set status alone until
  you unpin it.
- **Take off the calendar** — returns every booked release in the selection to
  the queue. Anything already released is left alone; its date is a record of
  what happened, not a booking.
- **Delete** — takes the whole batch to the [trash](/kilna/guides/the-trash/)
  and offers one undo for all of it, not one toast per work.

Each of these is one request rather than one per work, so the journal records
the batch as the single thing you did, and a failure cannot leave half the
selection changed with nothing to say where it stopped. If a work was already in
that status, or had nothing booked, it is counted as skipped rather than
reported as an error — that is why the number can be smaller than what you
ticked.

The same bar carries your profile's **AI actions**. A click asks that action of
every chosen work — each gets its own chat and its own answer, exactly as a
click on its own card would. Three run at a time and the rest wait their turn;
see [asking for many at
once](/kilna/guides/the-assistant/#asking-for-many-at-once).

**The selection is forgotten as soon as you act on it**, and is not remembered
across a restart. It is about the next click, not a state to keep.

## The row menu

The `⋯` at the end of a row opens what you can do with just that work:

- **Score it** — opens the work on its Score tab.
- **Plan a release** — opens it on Releases.
- **Delete** — to the trash, with the same undo.

Nothing in that menu opens the work by accident: clicks inside it stay inside
it.
