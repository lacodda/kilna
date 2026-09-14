# 0024 — A length is a number of seconds, and a board is timed in one change

Date: 2026-09-14
Status: Accepted

## Context

A storyboard is timed from the work it is a storyboard of: the duration
divides between the scenes proportionally, and the person drags from there
(decision of 2026-09-11). Two things stood in the way.

The length was not a number. `duration` shipped in the Music and Podcast
profiles as a `text` field holding `3:45` — a value a person reads and no
code divides. Nothing in the application had ever read it, so the shape was
never forced to be one thing: a workspace could hold `3:45`, `225`, or
"about four minutes" under the same key.

And timing touches every scene at once. Every edit so far has been one row:
a patch, a `before` beside it, an undo that puts that row back. Fifty scenes
timed as fifty operations would give the log fifty lines for one gesture and
the undo fifty steps to walk back through, leaving the board in forty-nine
states nobody asked for.

## Decision

**A length is a number of seconds, in the field it always was.** `duration`
is retyped to `number`, and migration 0018 converts what is already written —
`m:ss` and `h:mm:ss` become seconds, and anything else is left exactly as it
stands rather than guessed at. A second, numeric field beside the text one
was rejected: two truths about one length is what ADR 0001 forbids.

The migration carries both halves — the field's type in every stored profile,
and every value written under the key — because either alone leaves the
workspace inconsistent. `carry_forward` cannot do this: it matches vocabulary
by key and leaves a stored entry as the owner has it, which is exactly what
keeps a renamed or retyped field theirs. A workspace that had retyped
`duration` itself keeps its own version and is read leniently instead:
`duration_of` accepts a number, and still parses a time from a string.

**Timing a board is one operation, `scene.time`.** Its `before` carries the
span of every scene, including the scenes that had none, so one undo puts the
whole board back — including a span the person had set by hand before timing.
The division is even, because nothing on a scene yet says how long it wants to
be; the spans join, and the last ends on the length itself, so a rounded edge
cannot open a gap that v0.69's gap check would read as a hole nobody made.

**A work with no length is refused, not timed.** A board of fifty scenes all
starting at zero is worse than an untimed one, and the refusal names where to
give the work a duration.

Rejected: weighting the division by a scene's description or block count
(length is not verbosity); timing from an audio file (the file is not there
until assets arrive in 0.67, and the field is the truth either way);
timing scene by scene through `scene.update` (the undo above).
