---
title: History
description: The record kilna keeps of what you did — the feed, the bell, a work's history on one axis, and what gets swept away.
---

kilna writes down what happens to your work. Adding a work, renaming one,
saving a version, scoring, planning a release, marking one as released,
deleting anything and undoing it — each leaves one line in **History**.

The point is not bookkeeping. It is being able to answer *what happened to
this, and when* three weeks later, when the toast that said so has long
faded.

## The feed

**History** in the sidebar shows everything that happened in the active
profile, newest first. It is read-only. Nothing here can be edited or taken
back from the feed — a record you can rewrite is not a record.

Each line starts with a small tinted mark that says what kind of thing
happened: green for something that landed (a score, a release that went out),
the accent for something made or proposed, amber for something that moved or
wants a look, grey for something deleted, undone or tidied. Under the sentence
a quieter line names what it was about — a release, a version, the assistant —
and the day and time sit at the end. A dot on the left marks the lines that
still need a look.

Two filters sit at the top:

- **Everything** — the full list.
- **Needs a look** — only the entries that are asking for attention, and only
  while they are still unread. The filter counts them, so the head says there
  is something waiting before you switch to it.

**Mark all as seen** clears the second list without deleting anything from the
first.

## The bell

The bell in the top bar counts only entries that need a look, never ordinary
ones. Adding a work or saving a version is recorded but never lights it: a
bell lit by everything is a bell nobody reads.

Two things light it today:

- **A release losing its calendar slot** to a stronger work. That is the case
  where something happened *to* you rather than because of you, and the
  message announcing it was on screen for a few seconds while you were busy
  doing something else.
- **A release due inside the coming week that is not ready** — missing a
  required version role, or a score. kilna checks at startup and after every
  calendar change, and writes one warning per release and date.

A warning that happens again after you marked it seen comes back unread — a
repeat is news even when the first one was dismissed. The not-ready warning
deliberately behaves the other way: the same standing gap
noticed again at the next startup stays exactly as you left it, because a
bell relit every morning by the same unfinished work is a bell you stop
reading. Moving the release to a new date is a new situation and warns
afresh.

## Repeats are collapsed

When the same situation is recorded more than once, kilna updates the existing
line instead of adding another. The line moves to the top with a count under
it — `3 times` — so a recurring problem reads as one recurring problem rather
than filling the feed with copies of itself.

## A work's history on one axis

Every work's card has a **History** tab: the whole story of that work on one
axis, latest first. When it was begun; every version, and the one it was
written from (`Lyrics v4 · from v2`); every score and the version it read;
every publication made from it — the clip, the audio, a short — and the day
each went out, or is booked to. What is booked for a day after today stands
above today, under **Ahead**. The days are headed where you are: an evening
in another time zone falls on the day you lived it.

The axis is read off the work itself — its versions, its scores, its
releases — not off this feed. That matters twice. A work brought into kilna
from elsewhere arrives with its versions and scores and not one line about
them, and its axis is whole anyway. And a line of the feed you have read is
swept a week later (below), while a version is kept as long as the work is.

The feed adds what the work itself does not remember: a rename, a status that
moved, scenes put on the board, a proposal that arrived, a score or a version
deleted — what was taken out of a work is part of its story too. A line the
work already says — *version saved*, *scored*, *went out* — is shown once,
from the work.

Every moment leads to where it can be read whole: a version opens on the
Versions tab, a score on the Score tab, a publication on its own card. The
chips at the top take versions, scores, releases or the feed's lines off the
axis and back; the beginning stays. The overview's **Recent** widget shows
the latest of the same moments.

Entries survive what they describe. A line about a work you later deleted still
names it — the title is copied into the entry when it is written, not looked up
when it is read.

## Language

An entry stores *what happened*, not a finished sentence: an action and its
values. The wording is built when you read it, in whatever language the
interface is set to. Switch kilna to Russian and history written in English
reads in Russian, including entries from months ago.

The one thing that does not change is your own vocabulary. Statuses, kinds and
axis names come from your profile, and kilna does not translate the words you
chose — the same rule as everywhere else in the interface.

## What gets swept

History is the one place in kilna where something is removed for you:

- **Entries you have seen** are dropped seven days after they were written.
- **Entries you have not** are never dropped, however old.

That is the opposite of the [trash](/kilna/guides/the-trash/), which never
discards anything on its own — and deliberately so. The trash holds things you
might still want; history holds notices, and a notice you have already read and
acted on has done its job. A notice you have *not* read has not, so it stays
until you look at it.

The sweep runs when kilna starts. There is no scheduler and nothing to
configure.

## History belongs to a profile

Like works, notes and the trash, entries are scoped to the profile they were
written in. Switching profiles shows that profile's history.
