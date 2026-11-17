# 0082 — A gesture is undone whole, and an undo is replayed

Date: 2026-11-17
Status: Accepted

## Context

Since v0.83 one thing a person asks for is one unit of work and one
transaction (ADR 0040), but it may be several operations in the log: keeping
an assistant's package records a work, its versions and each scene of its
board; making a clip from a song records the clip, its link and its release.
Undo read the log's last operation and took back that one. After a package,
`Ctrl+Z` removed the last scene and left the work; after "make a clip", it
removed the clip and left a release nobody could see.

The replay had the opposite hole. It knew no `undo.*` operation at all, so
every undo went to `report.unknown` and a workspace rebuilt from its log
still held what had been taken back. An undo that buries a row also minted
the trash entry's id on the spot, so even a replay that knew undo would have
buried the row under another id, and a later `trash.restore` naming the
original would have found nothing.

## Decision

**The operations of one outermost unit share a gesture id.** The unit mints
it when the first operation inside it is recorded and forgets it when it
closes; `operation.gesture_id` holds it (migration 52). An operation recorded
outside any unit, and every operation logged before this, has none and stands
alone. Nothing declares a gesture: being one transaction is what makes
several operations one.

**Undo offers the newest gesture and takes it back whole**, newest operation
first, each in a savepoint of its own, all in one unit. Each reversal leaves
its own `undo.<kind>` line naming the operation it took back, so the log
stays one entry per thing done. The offer says how many more go with the
operation it names.

**A gesture holding an operation that cannot be reversed is not offered.**
Half a gesture taken back is a state nobody asked for: the work gone and the
file it carried in still there.

**What an earlier step already took is skipped, not failed.** Sending a work
to the trash takes its release and its links with it; reversing their
creation afterwards finds nothing, which in a gesture of several is the
expected state rather than an error. Nothing is logged for a step that did
nothing.

**An undo records what it minted, and the replay reverses with it.** The
undo's line carries a seed; the ids a reversal mints - the trash entries of
what it buries - are drawn from it in order. The replay reads an `undo.*`
operation, finds the operation it names in the same log, and calls the same
reversal at the undo's moment with the undo's seed, with the profile resolved
by key as every replayed operation's is.

## Consequences

- One press takes back a package, a derived work, a settled section of the
  canon. `Undoable.also` carries the count for the sentence.
- An undo that used to be offered for the last operation of a gesture with an
  irreversible one in it - a package that brought pictures - is no longer
  offered. The trash is still there for the work.
- A unit that wraps several unrelated gestures is one gesture to undo. That
  is what the unit already said about them to the database.
- A rebuilt workspace matches its source across an undo, and a restore from
  the trash logged after an undo finds its entry. The traces of deletions are
  written by the database at its own clock and are compared by what they
  bury, not when.
- An undo logged before v0.114 carries no seed: it is replayed, but its trash
  entries get fresh ids, as they had no recorded ones to begin with.
