# 0030 — A door belongs to the work that goes through it

Date: 2026-09-18
Status: Accepted

## Context

The shipped Studio profile gave a song three doors: `clip` "Video clip",
`short` "Short" and `audio` "Audio release". That was the whole model when
the profile was written: a song was the only kind of work, and "this song
goes out as a video" was a fact about the song, stated by a release row of
kind `clip` hanging on it.

It has not been the whole model since v0.57. A video is a work of its own
(ADR 0017), made from the song and linked to it in the `donor` role (ADR
0019), judged on the cut rather than the hook, shipping through doors of
its own — `youtube`, `premiere`. A short is the same one kind further (ADR
0029): a work with a donor and a list of stretches, shipping through
`short`.

The song's `clip` and `short` doors survived that change untouched, and
they said the same thing a second time, on a row that had no relation to
the work it was about. A `clip` release on a song meant "this song goes out
as a video" with nothing to say *which* video; its `requires` named the
song's roles — lyrics and a style prompt — as if those were what a video
needs to ship. Its fields asked for a pinned comment under a video that the
row could not name. The key `short` meant two things in one document: a
kind of work, and a door on a different kind of work. And a person planning
a month saw two chips with the same glyph for one thing, one on the song
and one on the video cut to it, with the calendar unable to say which was
the plan and which the echo.

## Decision

**A door belongs to the work that goes through it.** A song ships as an
audio release, and that is its one door. A clip is a Video work with the
song as its donor, and it ships through `youtube`. A short is a Short work
shipping through `short`. The shipped profile lists only `audio` under the
song and the instrumental; the video and short kinds keep the doors they
already had.

**An existing workspace is carried over the line once, the first time it
opens.** Every `clip` or `short` release hanging on a song or an
instrumental is moved onto a new work of the matching kind — a Video for a
clip, a Short for a short — created with the song's title and overview
fields the way *Make a Video from this* creates one, linked to the song as
donor at the song's current version. The release keeps its date, its time
and zone, its pin, the day it went out, its link and the text it went out
under; only the work it hangs on and the door it goes through change. One
new work per release row, not per song: a song with two short releases
becomes two Short works, because each short is its own piece. A work whose
release has already gone out arrives finished, and the statuses of both the
new work and the song are derived again from the facts, stepping over a
status a person pinned. Then the two doors come off the song in the stored
document, and one line in the history says how many releases moved.

**The tiers are left alone.** A song's `clip` tier — the band a total
lands in — is a judgement of the song, not a door, and a song can still be
good enough for a clip without any clip existing yet.

**A profile the owner invented is not touched.** The rule is about the
doors kilna itself shipped. A door on a song in a profile of the owner's own
is theirs, whatever it is called. And a release with nowhere to go — the
`video` kind deleted from the profile — is left where it is, door and all,
rather than failing the open.

**The move is a migration, not an operation.** It is not written to the
operation log, for the same reason the SQL migrations are not: it is what
the schema of this version means, not something a person did, and a replay
elsewhere runs the same migration on open. It is idempotent, and costs one
query when there is nothing to do.

## Consequences

A song's card shows one door. What was made from it — its clips, its shorts
— is on its Links tab, where it was already for anything made after v0.64,
and now for everything made before.

The calendar draws a clip once, on the video. The kind bar's *Short* chip
counts shorts, and nothing else, because only one kind of work ships
through that door now.

The moved works arrive without a plot and without a score: readiness marks
on their releases say so, honestly, where the song's row used to say
"ready" on the strength of lyrics that were never what a video needed. A
video that already went out has nothing left to be ready for.

A workspace that skipped several releases still arrives: the carry-forward
brings the video and short kinds in first, and the move runs after it on
the same open.

## Alternatives considered

**Keep the door on the song and read the Video work as "how the clip is
made".** The song would keep `clip` as a door and the video would be the
production behind it. Then a standalone video — one not cut to any song —
would have no door at all, because the door lives on the song; and `short`
would keep two meanings under one key, a kind of work and a door on another
kind. The model would have two kinds of video, one with a door and one
without, distinguished by whether a song happens to exist. Rejected.

**Relate the two with a `release.made_by_work_id` foreign key.** The
release would stay on the song and point at the Video work that produces
it. A second statement of "this was made from that" beside `work_link`,
kept in step by hand, and a release with two parents — the song it hangs on
and the video it points at — that every screen would have to choose
between. ADR 0001's rule against two truths in one place applies. Rejected.
