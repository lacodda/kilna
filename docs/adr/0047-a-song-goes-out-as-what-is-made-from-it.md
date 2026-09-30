# 0047 — A song goes out as what is made from it

Date: 2026-09-30
Status: Accepted

## Context

ADR 0030 took the clip and short doors off the song: a clip is a video made
from the song, a short is a short, each shipping through doors of its own. It
left the song one door, the audio release - and with it the same second
statement ADR 0030 was written against. An audio release on a video platform
has a title, a description, a cover in the shape of a video preview, a frame
it plays under and comments under it; a song has lyrics, a style prompt and a
score. Hanging the first on the second meant a song's card carried Releases
and Files tabs that were about something else, and a cover, a frame and
comments had no place at all.

The owner's decision of 2026-09-27: the song is the thing - its text, its
style, its score. What goes out is its publications - the clip, the audio,
the shorts - each a work of its own, with its releases, files, cover and
comments.

## Decision

**A kind may have no door at all, and the song has none.** The shipped Studio
profile gives `song` no `release_kinds`; a release planned on it is refused as
`release.noDoors`, which says where the release belongs instead of listing an
empty set of doors. The song's `instrumental` sibling is gone: an
instrumental is a variant of an audio release (the `variant` field, a choice
for `audio` alone), not a kind of work.

**A new kind, `audio`, is the track going out under a picture:** YouTube in
16:9 and streaming in 1:1, a cover prompt, a frame (ADR 0046), and the roles
`plot` (the concept) and `context` - the owner kept both for the audio
releases they made as videos before the kind existed.

**The shape of a cover belongs to the door** (`ReleaseKind.cover_format`):
YouTube 16:9, Shorts 9:16, streaming 1:1. A place added later is a kind of
release in the profile, with its own shape.

**A work with no door speaks through what was made from it.** Its status is
derived from its own releases and those of every work made from it, down the
links: a song is booked when its short holds a day and out when its audio went
out. A kind with doors speaks only for itself - a clip is not out because a
short cut from it is. A gesture that changes a fact restates the work and
every work up the chain it was made from; linking and unlinking restate both
ends; trashing a publication restates what it was made from. A status only a
person can say - "shelved" - is not overwritten even without a pin: it is a
decision the facts cannot improve on.

**Comments are kept under publications.** A comment on a work with no door is
refused (`comment.notAPublication`); the song's Comments tab sums up the
comments under everything made from it (`CommentFilter::under`), and its
counter counts them.

**Making a publication is one gesture.** *Make a clip / audio / short* names
the new work from the kind's `made_title` in the window's language, numbered
among its kind made from the same source ("· short 5"), gives it the source's
fields its kind has and the defaults of the rest, links it, and plans one
release through its first door with no day - then the window starts the
*release meta* action on that release and opens the cover.

**What a release goes out under is an action about the release** (`scope:
release`, `produces: release`). It reads the release and what is in it
(`{release}`), what the work was made from (`{source}`: the song's words and
fields), the canon a public text may see (`{canon:public}`, the channel card
first) and what went out lately (`{releases}`), and answers with a block of
fields. Started by the person, it fills the fields nobody has written; the
ones they started wait beside it as a proposal, taken whole or a field at a
time. An agent outside the window proposes the same shape (`propose_release`)
and fills nothing.

**The owner's workspace is carried over once**, the first time it opens,
while the stored song still has its audio door. Every audio release on a song
moves onto the song's audio work: one already made, a video named after the
song and the door ("— аудио-релиз"), which becomes an audio work, or a new one
titled as the song. Publications made before a link could say so and named
the way kilna names them ("— клип") are linked to their song. A publication
left holding two releases through one door is listed in the history for the
owner to settle, not guessed at. Then the door comes off the stored song, the
instrumental kind goes when nothing is of it, and every work without a door
is restated under the new rule. It is a migration, not an operation, for ADR
0030's reason. Rehearsed on a copy of the owner's workspace: 81 releases, 76
audio works made, 3 converted, 1 clip linked, 4 songs listed; 35 songs out,
as before v0.74 took the clips' facts away from them.

## Consequences

- A song's card has no Releases and Files tabs; its overview lists its
  publications with where each stands and a *Make…* menu.
- Tabs follow what the kind has: Versions where it names roles, Score where
  it names axes, Releases and Files where it goes out, Cover where it names
  cover blocks, Frame where it plays under one - and a tab stays while the
  work holds rows for it, so nothing already written is hidden.
- The calendar is unchanged: the audio's YouTube door is drawn with the disc
  the song's door had.
- ADR 0030's rule is now whole: a door belongs to the work that goes through
  it, and a work that goes through none is the thing the others go out for.
