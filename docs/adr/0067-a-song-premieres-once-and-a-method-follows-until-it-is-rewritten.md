# 0067 — A song premieres once, the title says so, and a shipped method follows until it is rewritten

Date: 2026-10-23
Status: Accepted

## Context

Two things the owner read on their own channel.

The texts a release went out under had one shape. Every description opened
with a scene, gave one exact detail, said "this song was written from...",
turned with a joke and ended with an invitation; every pinned comment named
the line or the moment the author loves most, put an emoji at the end of its
first paragraph and closed with "and you...?". Each text was fine; three in a
row read as machine-written. The shape came from two places: the channel's
card holds a description template as numbered slots (`hook`, `evidence`,
`song_entry`, `turn`, `cta`), and Studio's `release-meta` method told the
model to follow the template's order and to write the pinned comment "about a
line or a moment they love". The method also saw the wrong recent releases:
`{releases}` read the eight latest releases that went out before any that are
booked, and the owner's three newest texts were booked and not yet marked as
out - the model never saw them.

And an audio track that opens a new song should go out under a short title -
"Song (Song premiere 2026)" - not "Song - a line of it (audio)". The tail
" (audio)" was the field's `suffix` (v0.90, ADR 0051): one fixed string kept
at every write. Whether a release opens its song, and in which year, is not
fixed: it depends on its day and on the days of everything else made from
the song.

Underneath both: a method once stored in a workspace never changed again.
The upgrade fills a method that is missing (ADR 0021) and leaves a written
one alone, so a better shipped method reached new workspaces only.

## Decision

**A field may name a premiere's tail.** `ReleaseField.premiere` - one string
or a map per language, `{year}` the year it goes out - is the tail instead of
`suffix` when the release opens its song. The language is the title's own: a
title in Cyrillic letters takes the Russian word, any other the source
language. A tail is part of the title, so it speaks the title's language;
the window's language is not the channel's.

**A release opens its song** when nothing else made from what it is all made
from (`publication::origin`) went out while it still waits, or is booked for
an earlier day. A tie on one day is two openings. What holds no day is not
earlier than anything. A work made from nothing opens itself. **A kind may be
an excerpt** (`WorkKind.excerpt`, Studio's short): an excerpt never opens a
song and never stands in the way of one - a teaser out a day before the track
leaves the track the premiere.

**The tails are settled, not stored once.** `release_meta::keep_suffixes`
takes the release's standing (out or not, its day, the moment) and writes the
right tail: it takes off any tail the field knows - the plain one, a
premiere's in any language and year - and puts on the one that holds now.
Every writer that can change which release opens a song calls
`release_meta::settle` for the song's whole family after it writes: a release
made, its fields or day patched, booked, unbooked, marked out or taken back,
a layout applied or taken back, a shift applied or taken back. A release that
went out keeps the tail it went out under - that title is on the platform -
and `settle` skips it. Because the result is a function of the rows and every
replay and undo goes through the same writers, a replayed or undone gesture
settles the same.

**A method follows the shipped wording until the owner rewrites it.**
`METHODS_BEFORE` keeps every wording an action's method shipped with
(`src/profile/methods_before/`); a stored method still reading exactly like
one of them follows the shipped one at the next start. A rewritten one is
the owner's and stays.

**`{releases}` reads by day.** The eight releases closest to now, by the day
each goes out on, booked or out, each saying which. A text written for next
week, or one not marked as out yet, is as recent as its day.

**Studio's method treats the template as moves.** The channel's template is a
set of moves to choose from; a release takes two or three, no two running in
the same order. The model reads the recent releases as shapes and builds one
that repeats none of them; "this song is about..." and the favourite line in
a pinned comment are named as the habits a machine reaches for first, kept to
one release in five and never two running. The title is the song's name
alone; the tail is the application's.

## Consequences

- A release that went out under " (audio)" before v0.100.1 keeps it. One that
  is booked takes the premiere the next time it or a sibling is written.
- The tail is settled by writes. A sibling that leaves the workspace through
  the trash does not settle the rest; their next write does.
- A third tail - a re-release, a remaster - is a third field word, not a
  setting: the premiere is the one case the owner's channel names.
- The channel's own template on the card is the owner's: a proposal through
  `propose_canon` rewrites it as moves, and stays a proposal until they apply
  it.
