# 0051 — A publication goes out once

Date: 2026-10-02
Status: Accepted

## Context

ADR 0047 made a song go out as what is made from it: a clip, an audio
release, shorts - each a work with its releases, files, cover and comments.
"Its releases" stayed plural, and the owner's workspace never used the
plural: 193 publications, 193 releases on 2026-10-01. A clip on YouTube and
the same clip as a premiere are two things an audience meets in two places,
with two titles, two sets of numbers and two threads of comments; a second
release on one work had nowhere to keep any of that apart, and the Releases
tab drew a list that was always one row long, folded, to be unrolled before
anything could be done.

Three more things leaned on the plural:

- **`release.title`** - a third word for what a publication is called,
  beside the work's title and the title field of its door (`meta.title`).
  The first imports wrote it; no screen read it once the fields arrived. On
  the owner's workspace 105 of 188 said something else than the field.
- **Names.** kilna named what it made "Song — clip", "Song · шортс 3", in
  the window's language. 149 shorts in the owner's workspace were named after
  the song alone and could not be told apart; a Russian name in an English
  window was a different name.
- **A release title read the publication's name** (`{title}`), so a name
  kilna gives for the catalogue - "Song (audio)" - would have gone out to the
  audience.

The owner's decisions of 2026-10-01: a publication is its release; another
place is another publication; names by kind, one string for both languages;
an audio's title on YouTube ends with " (audio)", a clip's does not.

## Decision

**A work has at most one release** - a unique index on `release(work_id)`,
not an agreement. A second one is refused in words
(`release.onePerPublication`). Migration 0035 carries a workspace over
without loss: the work keeps its first release, each other one becomes a
publication of its own - the same kind, the fields, the cover and the frame,
made from what the work was made from, its files with it. `release.title`
goes: where the door's title field is empty the old word moves into it;
where the field says something else the old word stays beside it as
`meta.former_title`.

**The place is chosen when the publication is made** ("Make… → Audio →
Streaming") and may change until it goes out; once out, it is refused
(`release.placeAfterOut`). A tail a door keeps comes off when the release
moves to a door that keeps none.

**The release lives on the publication's overview.** The Releases tab is
gone; the overview's lead holds the release whole: the place, the day and
the hour (the time of day was in the model since v0.40 and nowhere in the
window), whether it went, the link, the fields with their generation, what
is proposed for them, the files. A song's publications widget shows each
publication's own place, day and hour.

**Names are the kind's `made_title`, one string for every language:**
"{title} (video)", "{title} (audio)", "{title} (short)". `{title}` is what
the publication is all made from - the song, even for a short cut from its
clip (`publication::origin`, one function where the cover and the idea board
kept a copy each). A template without `{n}` numbers from the second, inside
its closing bracket: "Song (short 2)". The number counts the works of the
kind made from that song, and a title already taken is skipped.

**What goes out under a name reads the song's, not the publication's:**
`{origin}`. Not `{donor:title}`: `{donor:x}` is a role of the nearest donor,
and the origin is up the chain.

**A tail is the field's**: `ReleaseField.suffix`, kept by the release
itself at every write of its fields or its door
(`release_meta::keep_suffixes`, called from `release::create_minted` and
`release::update_at`), so a title typed by hand, generated, proposed by an
agent or replayed from the log ends the same way. Within a limit the tail is
what stays; an empty field stays empty.

**The owner's workspace is renamed once**, the first time v0.90 opens it,
while a stored kind still names its works the pre-v0.90 way
(`publication::upgrade`): every publication made from something, numbered
in the order they went out; every field with a tail given it; and each
title that already went out listed in the history as "change it where it
went: was → now", because kilna does not reach into a platform. A kind the
owner renamed keeps the owner's names. The upgrades of v0.74 and v0.86 are
gone: after migration 0035 their precondition - two releases on a work - is
not a state a workspace can be in, and no workspace older than v0.86 exists.

The auto-layout's scatter rule follows: what used to be two releases of one
song are now two works made from it, so "never on neighbouring days" is
read by what a work is made from - links and cuts - not by the work alone.

## Consequences

- MCP `propose_release` names no release: the work names it.
- A blog post, a chapter or an episode that goes out to two places is two
  publications, the second made from the first.
- A field of the work's own media is not carried into what is made from it
  (`MetaField.own`): a short is not as long as the clip it is cut from.

Rejected: keeping the plural and drawing the list unrolled (the owner never
had two, and every other part of a publication - cover shape, comments,
numbers - is per place); a column for the tail on the release (a tail is a
rule of the place, so it is the profile's); renaming by a SQL migration (the
names are the profile's templates, read in Rust).
