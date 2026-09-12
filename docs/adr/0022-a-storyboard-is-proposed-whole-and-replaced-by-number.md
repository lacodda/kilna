# 0022 — A storyboard is proposed whole, and replaced by number

Date: 2026-09-12
Status: Accepted

## Context

ADR 0020 made a scene a row of the work's storyboard, with prompt blocks
keyed by the profile. ADR 0018 let an agent outside the window propose a
whole work as one package. The one thing an agent could not do was the
thing the storyboard exists for: write it. A video's plot and context are
versions and took the usual road (`propose_version`); the board itself had
to be typed in, row by row, from a note the agent left.

Two questions had to be settled. What the proposal is — a package with
scenes in it, or its own thing. And what *replacing* a board means for the
rows already on it, given that the rows will be pointed at (assets, v0.65)
and are in the trash and the undo log like everything else.

## Decision

**Scenes travel as `PackagedScene`, checked against the kind at the moment
of proposing.** The shot type and the block keys are the kind's own words,
the span is a span, the number starts at 1; anything else refuses the
whole package before it lands, the way an unknown role refuses a package.
A kind with no storyboard takes no scenes. The check runs in the server,
not at apply time, so the agent reads the refusal with the kind's words
listed and tries again, rather than the person finding a button that does
nothing.

**A storyboard is its own proposal, `scenes`, with a `replace` flag; a
package may carry scenes too.** `propose_scenes` proposes a board for an
existing work and the button says which decision it is — *add to the
board* keeps what is there, *replace the board* does not — because the two
are not the same click. `propose_work` takes `scenes` for a new video, or
adds them to an existing one's board; replacing is never a package's side
effect. The message body is the board as a table with every block under
it, so it is read before a row is written.

**Adding numbers after the last; replacing matches by number.** On a
replaced board, a proposed scene whose number a standing scene holds is
rewritten in place — every field, the blocks as a set — and **keeps its
id**; standing scenes the new board does not number go to the trash;
numbers nobody holds become new rows. Rebuilding the board from scratch
would make every row a stranger to whatever pointed at it, and a redo of
the chorus is not a new video.

**Every row is its own operation.** Applying writes `scene.create`,
`scene.update` and `entity.discard` exactly as the Scenes tab's own
buttons do, with the same params and the same journal lines, so a replay
cannot tell a proposed board from a typed one and undo walks it back a
scene at a time. A replaced board is therefore not one Ctrl+Z, and the
button says so.

Rejected: one `scenes.replace` operation holding the whole board (a second
way to write a scene, invisible to the coverage and undo gates that know
the first); replacing by deleting everything and creating anew (the ids
above); a tool per role for the plot and the context (they are versions,
and `propose_version` already proposes one).

## Consequences

- `Proposal` gains a `scenes` variant and `Proposal::Work` a `scenes`
  field; `Outcome` reports `scenes` written and `removed_scenes` (trash
  entries). The frontend counts both when deciding whether to offer a
  single undo.
- The journal gains `proposal.scenes`; the operations log gains nothing.
- The shared-context and plot roles stay versions. Actions on the Scenes
  tab that produce a board from inside the window (v0.63) will reuse the
  same proposal and the same apply path.
