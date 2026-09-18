---
title: The catalogue
description: The one list of everything you are making — sorting, filters that find unfinished work, and acting on several works at once.
---

The catalogue is every work there is: find one here, open one here. There is
no second list. Adding one is the **New** button in the title bar, which
reaches from any screen.

Each row carries the work's title, its status and kind, the tier and score it
last earned, and when it was scored. Clicking a row opens it.

## Choosing the columns

The **columns button** above the table, to the right, says which columns it
draws. It opens with the title, the stage dial, the marks, the tier, the score,
when it was judged and when it last changed; beside those there are three more:

| Column | What it says |
| --- | --- |
| ID | The work's identifier, shortened — what a plugin or the assistant calls it |
| Versions | How many versions the work holds, across every role |
| Added | When it was first written down |

The title cannot be turned off: it carries the row's identity and the link into
the work.

**Your choice survives a restart**, like the sort and unlike the filters. It is
how you prefer to read the table, not what you are doing this minute. It is
kept on the profile, as
[`catalogue_columns`](/kilna/reference/profile-document/#catalogue_columns):
a novel and a record are read down different columns, so switching craft
switches the columns with it, and a second machine opens the same profile on
the same table. A workspace from before this field opens on what the machine
remembered and writes that list to the profile once.

**Each kind of work can be read down its own columns.** Narrow the catalogue
to a kind and choose columns: the choice is kept for that kind, as
[`catalogue_columns_by_kind`](/kilna/reference/profile-document/#catalogue_columns_by_kind),
and the shared list stays as it was. A kind you never chose columns for reads
down the shared list. Videos will grow columns of their own — scenes, frames —
as the video stages land; the place for them is already there.

A mark your profile no longer defines is not drawn, the same rule the card
follows: renaming a mark never touches a work.

With enough columns on, or a narrow enough window, **the table scrolls
sideways** rather than squeezing its cells: the bar sits at the bottom of the
window, and the column headings stay put as you scroll down.

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

**The kind of work** sits above the search box as a row of chips — **All**,
then each kind your profile has, with the count of works behind it — as
soon as the profile has more than one kind. It is the mode the catalogue is
in: songs, videos, shorts. Choosing a kind shows only those works and stops
naming the kind on every row; choosing it again goes back to all of them.
The row is the same filter as `kind:` in the box, so a saved view keeps it.

The search box matches **anything the work says**, not only its title: the
lyric, the notes on it, the craft fields, what the assistant said about it. So
*which of my songs mention a fridge* is a question you can type — `холодильник`
— and typing two words narrows to the works that say both. Titles keep
narrowing as you type, and the rest arrives a moment later; see
[Finding things](/kilna/guides/finding-things/#a-word-finds-the-forms-of-a-word)
for how a word is matched.

The two dropdowns narrow by status and tier.

**The star** beside them shows only the works you starred — the ones to
come back to. A star is raised and lowered with a click, on the row or on
the card's header, and undo takes it back like any edit; it derives
nothing and is not a word of the profile, which is why it is a chip and
not a token in the box.

**The dial** in its own column says how finished each work is, as its author
judges it — an empty ring for one nobody has judged yet. Click it to set the
stage from the row; the same dial sits beside the star on the card, on the
calendar chip and on the dashboard. It is not the status: a work is routinely
*Scored* and still *Polishing*. See
[`stages`](/kilna/reference/profile-document/#stages) for the stops and how to
name your own.

**Each row says where the work stands** as a badge in the status's own
colour — *Scored*, *Scheduled*, *Released* — and what it is in outline
until the catalogue is narrowed to a kind. The colours are the profile's
([`statuses[].colour`](/kilna/reference/profile-document/#statuses)); a
status without one reads in outline. Marks in their column carry their
glyph and colour the same way the card's chips do.

### Narrowing from the box

The box also reads operators, so a slice you would otherwise click together
from three dropdowns is one line:

```
tier:clip status:draft winter
```

| Operator | Narrows by |
| --- | --- |
| `status:` | Your profile's statuses |
| `kind:` | Your profile's kinds of work |
| `tier:` | Your profile's tiers |
| `tag:` | A tag you put on the work yourself |
| `stage:` | How far along the work is — a stop's name, or its number |

Anything that is not an operator searches the full text. **Quote a phrase** to
keep it together: `"paper boats" tier:clip`.

A few things about how it reads what you type:

- **The word on screen works as well as the key.** Your tiers might be keyed
  `clip` and `pic` while the screen says *Clip* and *Picture*; `tier:Picture`
  finds the same works. Case never matters, in any alphabet.
- **A tag is matched whole.** `tag:win` does not find *winter* — tags are words
  you chose, so the exact one is the one you mean.
- **A stage is named or numbered.** `stage:polish`, `stage:Polishing` and
  `stage:80` all mean the same stop. The line is written back as the number, so
  a saved view keeps meaning the same thing if you rename the stop later.
- **A colon you did not intend as an operator stays in the search.** A work
  called *Ratio: a love song* is found by typing its title, because `Ratio` is
  not a field kilna knows.
- **A value your profile does not have is said out loud**, under the box,
  rather than left to look like a search that found nothing.
- **The box and the dropdowns are the same filter.** Pick a tier from the
  dropdown and the box says `tier:clip`; clear the box and the dropdown clears
  with it. The gap chips are the exception — they have their own row and are
  not written into the line.

### Views: a slice worth keeping

*Unscored clips*. *Winter songs still in draft*. The questions you ask the
catalogue every week are worth one click rather than four.

**Save this view** keeps what is on screen — the filter, the sort and the
grouping — under a name you give it, as a chip above the table. Click the chip
to put the catalogue back exactly as it was; click the `×` on it to forget it.

- The chip lights up while the catalogue matches it, and goes quiet the moment
  you change anything. What is highlighted is always what you are looking at.
- **Saving under a name you already used replaces that view** rather than
  making a second one beside it. That is how you adjust one.
- Views survive a restart, unlike the filters themselves: a named question is a
  standing one. They are kept per machine for now — the columns moved onto the
  profile in v0.50, and views follow when a version has a reason to move them.
- The chosen columns are not part of a view. They are how you read the table
  everywhere, not a property of one question.
- Twelve at most — past that the bar stops being a short row of standing
  questions and becomes a second list to search through.

Views are drawn on the catalogue rather than in the app's left rail: the rail
lists the screens, and a slice of the catalogue named there while the calendar
is open would point at something not on show.

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

A bar appears above the table with your profile's AI actions, and an
**Actions** menu holding the ordinary ones:

- **Move to status** — a submenu of your statuses; sends the whole selection to
  the one you pick. Chosen by hand, so each one holds there: the automation
  leaves a hand-set status alone until you unpin it.
- **Take off the calendar** — returns every booked release in the selection to
  the queue. Anything already released is left alone; its date is a record of
  what happened, not a booking.
- **Delete** — apart, below a separator. Takes the whole batch to the
  [trash](/kilna/guides/the-trash/) and offers one undo for all of it, not one
  toast per work.

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
