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
and the taken slots), **History** (what has happened, newest first),
**Trash** (everything you deleted, and the way back), and **Settings** (data
in and out, the profile editor).

Some entries are doors that are not built yet — Dashboard, Collections,
Notes. They sit in the sidebar with a small version chip naming the
release that delivers them, so the map of what is coming lives in the app
itself rather than in a changelog.

The footer holds the theme switch, the language switch, the profile picker,
and Settings.

## The topbar

The topbar names the screen you are on, counts your works, holds the search
box, and holds the **bell**.

**Search** takes `Ctrl+K` from anywhere and looks through titles, drafts,
notes and assistant replies at once — see
[Finding things](/kilna/guides/finding-things/). The bell is lit only by entries that need a look — never by the
ordinary record of you adding a work or saving a version, because a bell lit
by everything is a bell nobody reads. Clicking it opens
[History](/kilna/guides/the-history/).

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

Opening a work gives you a card with a cover, a bar and seven tabs.

**The cover** is a gradient derived from the work's id. It is not decoration
you chose; it is there so one card is distinguishable from another before you
have read a word, and it stays the same for as long as the work exists. Real
covers replace it later. The way back to the list sits on it — the one part of
the card carrying nothing else, and on a narrow window the list beside it is
gone entirely.

**The bar sticks.** The cover scrolls away, but the work's name, its status,
its tier and score, the profile's own fields, and the tabs stay at the top —
a long version otherwise leaves you reading with no idea whose words they are.
The name comes first and the craft's numbers under it: BPM and key are
reference you consult, not what you identify the card by. Everything in that
bar is read-only. You edit it on **Overview**, because a header you can type
into is a header that shifts under the cursor while it saves.

**The trail at the top** names where you are: *Works › Harbour lights*, with
the first part a link back.

**The tabs:**

| Tab | What is there |
| --- | --- |
| Overview | The title, status, kind, and the fields your profile defines |
| Versions | Every draft, by role, the editor, and comparison |
| Score | The axes, and what this work has scored before |
| Releases | What ships, where and when — with a count on the tab |
| Notes | Notes attached to this work |
| Assistant | The AI panel for this work |
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
