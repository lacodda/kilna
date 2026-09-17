# 0029 — A short is a work with a donor and a list of stretches

Date: 2026-09-17
Status: Accepted

## Context

A short is made one of two ways: cut out of a finished video, or shot for
itself. The obvious reading of that sentence is that they are two things, and
the obvious design is a flag, or two kinds, or a second table for the cut
ones.

They are not two things. Everything a short has — a title, a score, a stage,
a storyboard for its titles, a cover, a door it goes out through — it has
either way. The `short` kind has stood in the shipped profile since v0.60
with all of it, and `work_link` has held a `donor` role since v0.64 (ADR
0019). What was actually missing was smaller and more specific than a second
model: nobody could say *where in the donor*.

The predecessor said it in a filename. A shorts queue was a folder of clips
named by hand, the boundaries lived in whatever command line produced them,
and the answer to "which part of that video is this" was gone the moment the
file was made.

## Decision

**The donor is the whole of the difference.** A short with a donor link was
cut from something; a short without one was shot. No flag, no second kind, no
column that says which sort of short this is — the fact is already recorded,
by the link, and a second statement of it is a second thing to keep in step.

**The boundaries are a table, not two columns.** `cut` holds one row per
stretch: the short, the source, the seconds in and out, its place in the
splice. A short is routinely spliced from more than one stretch of one video
— the line that lands, then the chorus eight bars later — and a `cut_from`
and `cut_to` on `work` can hold the first and is silent about the second.
That silence is the bad kind: the person gets no error, they get a product
that quietly cannot hold what they are making.

Rejected: reusing `scene`, which already carries `starts_at` and `ends_at`.
A scene's seconds say where it plays in *its own* video — it is a row of a
storyboard, a prompt and a frame. A cut's seconds say where it was taken from
in *someone else's*. Putting both in one table makes what a row means depend
on whether the work has a donor, which is two truths in one place (ADR 0001);
and a short can legitimately have both at once, cut from a video *and*
storyboarded for its titles.

**The source is on the stretch, not on the work.** Denormalised from the
link on purpose: a short may be spliced from two different videos, so which
one a given stretch comes from is a fact about the stretch. The link remains
the statement "this was made from that", with the version it was taken at;
the column is which of them this stretch is.

**The core keeps the boundaries and never opens the file.** `cut_shot_list`
hands out the stretches in order, each beside the donor's video on disk, and
that is the whole of the core's part in making the file (decision of
2026-09-11: no ffmpeg in the core). A stretch whose donor has no video yet
comes back with no path rather than failing the list: a splice written before
the donor is rendered is an ordinary half-finished short, and the screen says
which line is missing its file.

**The calendar's spacing rule reads the donors.** Two shorts cut from one
video on consecutive days are the same video twice to anyone watching,
whatever the two shorts are called, so the scatter rule that already keeps
two releases of one work apart is asked the same question on a second key.
Not a separate queue and not a seeded shuffle (decision of 2026-09-11): a
rule that holds every day beats an order that happens to look spread out, and
the layout stays a function of the facts alone.

**A cover's prompt is a body with named parts, kept on the work.**
`work.cover` is a JSON object keyed by the kind's new `cover_blocks` — the
same shape `scene.blocks` has, because it is the same thing one level up:
text a person edits and copies a part at a time. Not three columns, because
the code is not allowed to know what a cover is made of (ADR 0001) — a craft
whose covers carry a fourth part says so in its profile. Not `work.meta`,
because meta is the row of craft *fields* under the title and a prompt is a
body; a paragraph in the field row breaks the row as surely as it hides the
prompt. Not rows in `scene`, because a work has one cover and a storyboard
has many scenes.

## Consequences

A short is read from either end. Its own card says which stretches of what it
is; the donor's card says what has been taken out of it, and at which
moments — a question the predecessor could not ask at all.

The cutting plugin of v1.10 has a seam to arrive at, and nothing has to
change here when it does: it reads the shot list and writes files.

The calendar spreads a batch of shorts over their sources without being told
about shorts. A studio that cuts trailers out of films gets the same
behaviour under its own words, because nothing in the rule names a kind.

The cover prompt is indexed with the rest of the work, so the one a person
remembers by its picture — "the one with the neon corridor" — is findable by
the words that produced it.

A workspace that predates this gains the cover's parts through
`profile::carry_forward`, by name, the way `scene_blocks` arrived: a field
added to the shipped profile reaches nobody otherwise, because every
workspace alive already has its kinds.
