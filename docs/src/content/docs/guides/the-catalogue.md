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
draws. It opens with the title, the marks, the status, the stage dial, the
vibe, the total, the versions and the next release; beside those there are
more:

| Column | What it says |
| --- | --- |
| ID | The work's identifier, shortened — what a plugin or the assistant calls it. A press copies the whole id |
| Total | The score and the tier it earned, as the tier's letter by rank - S for the highest, then A, B, C. The tier's word, the day it was scored and how far the next tier is are on hover; a turned arrow says the work was edited after its score, a pin that the tier is held by hand |
| Next release | The day of the next booking, after a dot that says whether it can go out: green when it can, amber while something is missing, red once the day is close - what is missing is on hover. With nothing booked, the work's [planned slot](/kilna/guides/planning-a-release/#planned-slots), dashed and marked *plan* |
| Versions | How many versions the work holds, across every role; a quiet dash for a kind that keeps none |
| Collection | The collection the work is in |
| Tags | The first two tags and how many more |
| Made from it | For a song, a chip a kind for what was made from it - its audio, clips and shorts - green once all of them are out, blue while one is booked. A press opens the work, or a menu of them when there are several |
| Added | When it was first written down |
| Edited | When it last changed |

Until v0.140 the tier, the day it was scored and whether the release was
ready were columns of their own. They live in the total and the next release
now; a list of columns that still names them reads without them.

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

**The columns go in the order you put them.** In the columns menu the shown
columns come first, in the order the table draws them, each with a grip on
its right; drag a row up or down to move that column left or right in the
table, or, with a row focused, press **Alt + ↑** / **Alt + ↓**. A column you
turn on joins at the end, where it is found without looking, and is dragged
from there to where it belongs. The order is kept where the choice of columns
is kept — on the profile, and per kind when you chose columns for a kind — so
it follows the profile to a second machine the way the columns do.

**Column widths are dragged.** Every heading ends in a thin handle; drag it
to make the column wider or narrower, and double-click it to give the column
its natural width back. A column narrowed past its contents clips them with
an ellipsis rather than folding the row onto two lines — the full title is
in the tooltip. The title takes whatever width the window has spare, unless
you sized it by hand too. Widths are kept **per machine**, not on the
profile: the pixels that fit a title on a laptop are not the pixels that fit
it on a monitor, and a width that followed the profile would carry the wrong
answer to the other screen.

With enough columns on, or a narrow enough window, **the table scrolls
sideways** rather than squeezing its cells: the bar sits at the bottom of the
window, and the column headings stay put as you scroll down.

## Grouping

The **group** dropdown folds the table into blocks by status, by tier or by
[collection](/kilna/guides/collections/). Click a block's heading to collapse
it; the count beside the heading says how many rows are inside.

The sort still decides the order, so a catalogue grouped by tier and sorted by
score opens on the tier holding your best work. Works with no value — no tier
yet — gather in a block of their own at the bottom, for the same reason unscored
works sort last.

**Grouping is forgotten when you close kilna**, like the filters. It is a way of
interrogating the list today.

Grouped by collection, the works in none gather under *In no collection*, and
a collection's heading is a place to carry rows into it - see
[carrying rows into a collection](#carrying-rows-into-a-collection).

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
judges it: one of five stops - an empty ring at the start, an arc of a
quarter, a half and three quarters, the whole ring when it is final. No word
stands beside it; point at one and it says its stop, *75% — almost ready /
polishing*. Click it and five dials open in a row: press the one the work
stands at. The same dial sits beside the star on the card, on the calendar
chip and on the dashboard. It is not the status: a work is routinely *Scored*
and still at 75. See [`stages`](/kilna/reference/profile-document/#stages)
for the stops and how to word them.

**The vibe** beside it says what the work feels like, along three lines:
its **mood**, its **character**, and how **complex** it is. One mark is built
from the three - the mood's colour, the character's shape (a wave, a diamond,
a circle, a broken line, a chevron), as many strokes as it is complex - so
"melancholy, raw, complex" is a smoky-blue broken line in three strokes, and
a column of them is read at a glance. Point at a mark and it says its words.
Click it, or the quiet **+** of a work nobody has said anything of, and the
three lines open as words to press: say one, two or all three - a line left
unsaid stands at its middle word while the mark is drawn - and press a word
again to take it off. The same picker stands in the card's header beside the
stage, and the mark on a calendar's slot.

A clip, an audio release or a short with no vibe of its own shows the one of
what it was made from, drawn quieter, and says whose it is; saying one for
it makes it its own, and **Take it from what it was made from** gives the
source's back. The assistant reads a vibe when it scores a text, and an
agent may propose one: either is kept only where you have said nothing. A
free word under the **Mood** field of a song made before v0.124 is left as it
is. See [`vibe`](/kilna/reference/profile-document/#vibe) for the words.

**Each row says where the work stands** as a badge in the status's own
colour — *Scored*, *Scheduled*, *Released* — and what it is in outline
until the catalogue is narrowed to a kind. The colours are the profile's
([`statuses[].colour`](/kilna/reference/profile-document/#statuses)); a
status without one reads in outline. Marks in their column are small round
tiles, as on the card: each its glyph in its colour, its name on hover. Two
marks take the width of two icons rather than of two phrases. Pressing the cell - its tiles, or the dash
of a work with none - opens the card's menu of marks over it, and a mark is
raised or taken off without opening the work (v0.139).

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
| `collection:` | A [collection](/kilna/guides/collections/), by its name |
| `mood:` `character:` `complexity:` | What the work feels like - a word of the line, or its key; a work with no vibe of its own is found by the one it shows |

Anything that is not an operator searches the full text. **Quote a phrase** to
keep it together: `"paper boats" tier:clip`.

A few things about how it reads what you type:

- **The word on screen works as well as the key.** Your tiers might be keyed
  `clip` and `pic` while the screen says *Clip* and *Picture*; `tier:Picture`
  finds the same works. Case never matters, in any alphabet.
- **A tag is matched whole.** `tag:win` does not find *winter* — tags are words
  you chose, so the exact one is the one you mean.
- **A stage is named or numbered.** `stage:almost`, `stage:polishing` and
  `stage:75` all mean the same stop - a stop is found by its key, by either
  half of what it says, or by its number; `stage:idea`, `stage:start`,
  `stage:half`, `stage:almost` and `stage:final` (and `идея`, `старт`,
  `половина`, `почти`, `финал`) find theirs whatever you renamed them to. The
  line is written back as the number, so a saved view keeps meaning the same
  thing if you rename the stop later, and a number written on the older scale
  - `stage:83` - reads as the stop it lands on.
- **A collection is named, and held by what it is.** `collection:"Deep time"`
  finds the album; the box shows the name back, so renaming the album later
  keeps a saved view pointing at it. Two collections with the same name are
  told apart by an id in the line instead.
- **A colon you did not intend as an operator stays in the search.** A work
  called *Ratio: a love song* is found by typing its title, because `Ratio` is
  not a field kilna knows.
- **A value your profile does not have is said out loud**, under the box,
  rather than left to look like a search that found nothing.
- **The box and the dropdowns are the same filter.** Pick a tier from the
  dropdown and the box says `tier:clip`; clear the box and the dropdown clears
  with it. The gap chips are the exception — they have their own row and are
  not written into the line.

### Narrowing from a column

The headings that narrow carry a **funnel**, shown when the pointer is over the heading
and staying on, filled, while that column is narrowing the list:

| Column | The funnel asks for |
| --- | --- |
| Work | Words of the title, each matched anywhere in it and in any case |
| Stage | One or more of your profile's stops — works standing at any of them |
| Total | One or more tiers, each a range of the score - *S · Strong · 78 and up* - and **(not scored)** |
| Next release | **Planned** - a booking or a planned slot ahead - **Overdue** - its day passed and it is not out - or **No date** |
| Collection | One or more collections, and **(empty)** for works in none |
| Tags | One or more tags, and **(empty)** for works with none |
| Marks | One or more marks — works carrying any of them — and **(empty)** for works carrying none |
| Vibe | Words of each line, and **Nobody has said** - ticks on one line ask for any of them, ticks on two lines for both |

Ticking two stops means *either*, the way the status dropdown means one
status; the funnels on different columns combine as *and*, with each other
and with everything in the box. The Stage funnel is the five dials in a row,
each a tick: `winter` in the box with the last two pressed is "winter songs
that are nearly done", and the first dial alone is what nobody has touched
yet. A work with no tier is left out once tiers are ticked, unless
**(empty)** is ticked with them. **Clear** inside the panel empties that
funnel and puts its switches back; unticking the last box empties it too.

At the top of each funnel two switches say how its ticks are read (v0.139):

- **With | Without** - keep the works the ticks match, or drop them and keep
  the rest. *Without* "Working on it" is every work not being fought with.
- **Or | And** - a work matches if it has any of the ticks, or only if it has
  all of them. Two marks with *And* are the works carrying both; the Work
  funnel's words with *And* are titles holding every word.

Together they make four questions: with any, with all, without any (none of
the ticked), without all (not every one of them). A work holds one status,
one stop and one tier, so *And* over two ticks of those would ask for a work
in two places at once: the switch is closed while two are ticked, and says
why. The Vibe funnel has only *With | Without* - its ticks already read as
any word on a line, every line ticked.

Column filters are not written into the query line: a set of ticked stops
has no place in it. They have the query's lifetime — kept while kilna is
open, gone when it closes — and the count above the table, and its
**Clear**, take them into account together with the box. Opening a saved
view puts every funnel down, since a view is the whole question; the funnels
are not part of a view.

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

**Two more chips slice by repeats** (v0.111), when anything repeats:
**With repeats** keeps every work the
[guard of repeats](/kilna/guides/the-guard-of-repeats/) marks, **Red** only
those whose song says a rare word a song out or booked within the window
says. A clip, a short and an audio release are in the slice their song is in.
Like the gap chips they count the whole catalogue, and are not part of the
query line.

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

A bar appears at the foot of the table, under the rows, so ticking the first
one moves nothing you were aiming at. Each act is its own button:

- **Move to status** — a menu of your statuses; sends the whole selection to
  the one you pick. Chosen by hand, so each one holds there: the automation
  leaves a hand-set status alone until you unpin it.
- **Marks** — your profile's marks, ticked where every chosen work carries
  one and dashed where only some do. Pressing a mark raises it on all of
  them, or takes it off all of them if they all carry it; a work already that
  way is passed over. One undo puts each work's marks back as they were.
- **Fill "Code"** — for a profile with a
  [numbered field](/kilna/reference/profile-document/#numbered-fields), one
  button per such field: the chosen works that have no code are given the next
  codes, in the order they were made. A work with a code, or of a kind without
  the field, is passed over. One undo takes the codes back.
- **Take off the calendar** — returns every booked release in the selection to
  the queue. Anything already released is left alone; its date is a record of
  what happened, not a booking.
- **To collection** — puts the selection at the end of the collection you
  pick, in the order the table shows it, or in a new one made for it. A work
  already in that collection is passed over; one in another collection
  leaves that one. One undo takes the lot back, each work to where it stood.
- **Delete** — apart, at the end. Takes the whole batch to the
  [trash](/kilna/guides/the-trash/) and offers one undo for all of it, not one
  toast per work.

Each of these is one request rather than one per work, so the journal records
the batch as the single thing you did, and a failure cannot leave half the
selection changed with nothing to say where it stopped. If a work was already in
that status, or had nothing booked, it is passed over rather than reported as
an error — that is why the number can be smaller than what you ticked — and a
second note under the result names each work passed over and why: "already
that way", "nothing booked", "already has one", or what refused it.

The same bar carries your profile's **AI actions**. A click asks that action of
every chosen work — each gets its own chat and its own answer, exactly as a
click on its own card would. Three run at a time and the rest wait their turn;
see [asking for many at
once](/kilna/guides/the-assistant/#asking-for-many-at-once).

**The selection is forgotten as soon as you act on it**, and is not remembered
across a restart. It is about the next click, not a state to keep.

### Carrying rows into a collection

Press on a row and move the pointer: the row is picked up, and the bar at
the foot gives way to a **shelf** of your collections, each with how many it
holds, and a dashed **New collection**. Let go over one and the row goes in
at its end; let go over *New collection* and you name a new one with the row
already inside. Picked up by a ticked row, every ticked row on show goes
with it. While the catalogue is grouped by collection, a collection's
heading takes the rows too.

A press that does not travel is still a click, and opens the work. **Escape**
puts the rows back. The controls inside a row - the tick, the stage dial, the
row menu - answer their own presses and pick nothing up. The keyboard's way to
the same place is **To collection** in the bar.

## The row menu

The `⋯` at the end of a row opens what you can do with just that work:

- **Score it** — opens the work on its Score tab.
- **Plan a release** — opens its overview, where its release is planned.
- **Delete** — to the trash, with the same undo.

Nothing in that menu opens the work by accident: clicks inside it stay inside
it.
