---
title: The interface
description: How kilna's window is laid out — the title bar, the sidebar, themes, URLs you can navigate by, and how the app tells you what it did.
---

kilna's window is a frame around one working surface. The frame does not
change as you move: the title bar across the top, a sidebar on the left, and
the current screen filling the rest.

**The window itself never scrolls.** Each screen is handed the height that is
left and stops at the bottom edge; what is longer than that scrolls inside its
own box — the catalogue's table, the calendar's queue, the text of a version —
and the frame around it stays put. A desktop window has a bottom edge, and a
page that runs past it the way a web page does hides the fact that there is
more.

The window opens on a splash — the mark, the name, the version — for the
moment it takes to open the workspace, and it comes back the size and place
it was closed at: maximised if it was maximised, 1200×1200 if it was that.

## The sidebar

The sidebar lists the screens: **[Catalogue](/kilna/guides/the-catalogue/)**
(every work there is — add, search, filter, and open one), **Calendar** (the queue and the taken slots),
**History** (what has happened, newest first), **Trash** (everything you
deleted, and the way back), and **Settings** (data in and out, the profile
editor).

There is one list of works, not two. Until v0.21 a Works screen carried a
second list beside the open card; it said the same things as the catalogue
and made you choose which to look in.

Some entries are doors that are not built yet — Collections and Notes. They
sit in the sidebar with a small version chip naming the
release that delivers them, so the map of what is coming lives in the app
itself rather than in a changelog.

The footer holds the theme switch, the language switch, the profile picker,
and Settings.

**The sidebar folds to icons.** The handle at the left end of the title bar
switches it between the full menu (216px) and a column of icons (52px): the
words, the version chips and the profile picker go, the *Library* caption
becomes a thin rule, and each icon names its screen on hover. The choice is
kept on this machine rather than in the profile — how much room the menu takes
is a habit of the person at this screen, not a fact of the craft. Settings is sections under their own addresses — General, The
work card, Profile, Data, Agents — and *The work card* is where you choose
which tab a work opens on: Overview by default, Versions or Score if that is
where you live.

## The title bar

There is one bar rather than a system one over an application one, as in
scheda. From left to right: the menu handle, the mark and the version, where
you are (the screen, or *Catalogue › the work*), the **search box** in the
middle, the count of works, the **New** button, the **assistant**, the
**bell**, and the window's own buttons. The screens do not repeat their own
name under it: the trail already says where you are. On a narrow window the
name and version beside the mark and the count of works step aside and the
trail shortens; the window's buttons never leave. Everything that is not a control drags the window; a double-click
maximises it.

**New** adds a work: with one kind of work in the profile it asks for a title
straight away, with several it lists the kinds first. The catalogue used to
carry a title box for this at its top; the button reaches from any screen.

**Search** takes `Ctrl+K` from anywhere and looks through titles, drafts,
notes and assistant replies at once — see
[Finding things](/kilna/guides/finding-things/). The bell is lit only by entries that need a look — never by the
ordinary record of you adding a work or saving a version, because a bell lit
by everything is a bell nobody reads. Pressing it opens the last few entries
in place, the ones that need a look first, with *See all* leading to
[History](/kilna/guides/the-history/).

The whole shell answers to the keyboard: `G` then a letter jumps between
screens, `Alt+←` walks back the way a browser does, and `?` shows the list from
wherever you are — see [Keyboard](/kilna/reference/keyboard/).

One more thing floats over every screen: the **assistant button** in the
bottom corner, with a badge while runs are in flight. It opens a drawer with
every chat of the profile — see [The assistant](/kilna/guides/the-assistant/).

## Themes

kilna is a studio tool, so it is dark by default — more precisely, it
follows your system's theme until you say otherwise. The theme button cycles
through *system*, *light* and *dark*; the choice is remembered across
launches and applies before the first paint, so the window never flashes the
wrong color.

## Language

kilna speaks English and Russian. The footer button cycles through
*system*, *English* and *Русский*; like the theme, the choice is remembered
and applied before the first paint.

*System* follows your OS: kilna takes the first of your preferred languages
it has a translation for, matching on the language rather than the region —
`ru-RU` and `ru-BY` both get Russian.

**What is translated is the interface, not your vocabulary.** Statuses,
kinds, roles, scoring axes and meta fields come from your
[profile](/kilna/concepts/profiles/), which is data you own and can edit —
so switching the interface to Russian does not rename `Draft` to
`Черновик`. Rename it yourself in Settings and it stays renamed, in every
language. The alternative — translating your profile on the fly — would
overwrite whatever wording you had chosen.

There is no fallback between languages. A message missing from one locale
fails the build rather than appearing in the other one, because a screen
half in English is the kind of bug nobody reports and everybody notices.

## Screens are URLs

Every screen has an address, and the open work is part of it: `/works/abc123`
is that work, opened, and `/works/abc123/score` is its Score tab. The back
button walks your actual history — including between tabs of the same card —
and anything that can hold a link, a note, a chat message, a journal entry,
can point at a tab directly.

## A work's card

Opening a work gives you a card with a cover, a header and its tabs —
Overview, Versions, Scenes, Cut, Score, Releases, Files, Links, Notes,
Assistant, History; Scenes and Cut only where the work has them.

**The cover** is a gradient derived from the work's id. It is not decoration
you chose; it is there so one card is distinguishable from another before you
have read a word, and it stays the same for as long as the work exists. Real
covers replace it later. The way back to the catalogue sits on it — the one
part of the card carrying nothing else.

**The header stands still.** The cover, the work's name, its status, its tier
and score, the profile's own fields and the tabs stay where they are, and the
open tab takes the rest of the window and scrolls inside itself — a long
version otherwise leaves you reading with no idea whose words they are.
The name comes first and the craft's numbers under it: BPM and key are
reference you consult, not what you identify the card by. The fields there are
read-only, and long ones are cut short with the whole value a hover away — you
edit them on **Overview**, because a header you can type into is a header that
shifts under the cursor while it saves. **The row of craft fields is off unless
you ask for it** — Settings has the switch, under *The work card* — because a
row of eight truncated values is a poor way to read fields Overview lays out in
full. Turning it off hides the row and nothing else: the values stay, Overview
still edits them, exports still carry them.

**The name has its own row of things done to it.** The pencil turns the
heading into a box of the same height — Enter saves, Escape cancels,
leaving the box saves — so the header does not move while you rename. Next
to it: copy the title, the id (click it to copy), the **star** — *come back to
this one* — and the **dial**: how finished the work is, as you judge it. The
star is not a status and not a mark; the catalogue has a chip that shows only
the starred.

**The dial** opens a row of stops — *Idea*, *Rough take*, … *Finished* — and
clicking one sets it. The same dial appears in the catalogue's own column, on
a calendar chip and on the dashboard, so how far along something is reads the
same everywhere. An empty ring means nobody has judged it yet, which is not
the same as judging it a bare idea; clicking the stop it already stands on, or
`Backspace`, takes it back to unjudged. The stops are your profile's
([`stages`](/kilna/reference/profile-document/#stages)); a work that shipped
was set to finished when the workspace gained the dial.

**Under the name: marks and tags.** Marks are the flags your profile
offers — *Working on it*, *Not sure*, *The good one* — each with its glyph
and colour, raised and lowered with a click. Tags are your own words for
what the work is; the box completes from what the workspace already says, so
a vocabulary converges instead of scattering into near-misses. Neither moves
the work: the status above is worked out from what happened, and these are
what only you know.

**The menu at the end of the row** copies a link to the card, makes a work
of another kind from this one — a video from a song — and deletes the work,
which goes to the [trash](/kilna/guides/the-trash/) with an undo.

**A review sits beside what it reviews.** If your profile has a role that
comments on another — Music ships two, a read against the axes and a critique
of the lines — the Versions tab opens the text on the left and what was written
about *that revision* on the right, one tab per kind. Reading criticism away
from the lines it discusses is reading half of it. On the Score tab, the
revision beside a score is a link to exactly that: the draft that was judged,
with its review already open.

**The trail at the top** names where you are: *Catalogue › Harbour lights*,
with the first part a link back.

**The tabs:**

| Tab | What is there |
| --- | --- |
| Overview | The title, status, kind, the fields your profile defines, and its AI actions |
| Versions | Every draft, by role, the editor, and comparison |
| Score | The axes, and what this work has scored before |
| Releases | What ships, where and when — with a count on the tab |
| Notes | Notes attached to this work |
| Assistant | The AI panel for this work — see [The assistant](/kilna/guides/the-assistant/) |
| History | Everything that happened to it |

Only the open tab is loaded. Opening a card no longer fetches every version,
score, release, note and chat a work has ever had.

**Drafts belong to their role, and survive a closed window.** Text typed under
one role stays under it, switching back returns what you were writing, and
closing kilna does not lose it. See
[Writing a version](/kilna/guides/writing-a-version/).

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

The same event is also written to [History](/kilna/guides/the-history/), from
the same wording: the message is what you are told now, the entry is what you
can find later.

**Deleting never asks.** A deletion says what went and offers *Undo* in the
same message. kilna does not put a confirmation in front of it, because a
confirmation costs a click every single time to guard against the rare
mistake, while an undo costs a click only when the mistake actually
happened — and what you deleted is in the [Trash](/kilna/guides/the-trash/)
either way, long after the message is gone.

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
