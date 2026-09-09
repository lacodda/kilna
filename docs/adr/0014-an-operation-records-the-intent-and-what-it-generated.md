# 0014 — An operation records the intent, and what carrying it out generated

Date: 2026-09-09
Status: Accepted

## Context

ADR 0012 gave the schema a memory of what *became* of every row: a tombstone
under each deletion, a clock on each edited field, both written by triggers so
that no code path can forget them. It closed by naming what it did not do —
"the operations log planned for the next stage carries the full history, and
this table is not asked to."

Two gaps are left, and they are the same gap seen twice. A tombstone keeps only
the latest cycle for one id, so a row deleted, restored and deleted again
remembers one deletion. And nothing anywhere records *intent*: SQLite sees
three updates whether the person applied one status drift or edited three works
by hand, and `apply_layout`, `set_works_status`, `delete_works` and
`set_collection_contents` each turn one gesture into many rows. Intent cannot be
recovered from the rows afterwards, and it cannot be written by a trigger,
because the only layer that knows a gesture is the command that received it.

The log is also the last breaking change before 1.0 freezes the format, so the
shape chosen here is the shape kilna keeps.

## Decision

**An operation is the person's intent plus everything carrying it out
generated, written by the command layer, kept forever.**

Migration 0013 adds one table, `operation (seq, id, device_id, profile_id,
kind, params, recorded_at)`. `seq` is an autoincrement, and the order a replay
reads is `(recorded_at, seq)`: the timestamp is what two devices compare, `seq`
breaks the tie within one — a mass action writes faster than the millisecond
the stamps carry.

`params` holds both halves. The person's input, and the values the code minted
while acting on it: the uuid, the timestamp, the position computed from
`max(position) + 1`. Recording only the input looks tidier and does not work —
a replayed `work.create` would mint a *different* uuid, and every version,
score and release naming the original would part ways with the row it means.
So each domain function that mints an id or stamps a time takes a seam: the
value is supplied from outside when a replay supplies it, and generated as
before when nothing does.

**Why not a snapshot of the rows.** The trash (migration 0003) already stores
deletions that way — the row verbatim, restored by inserting it back — and it
would have been the cheaper build here, needing no seam anywhere. It is
rejected because of what 1.0 freezes. A snapshot of rows is a second body of
the database, and it is only replayable against the schema it was written
under: every later migration would have to migrate the log too, or leave old
operations unreplayable. An intent outlives a schema change — "the person
created a work titled X" still means that after a column is added — and a
format that has to survive the whole 1.x line has to be the one that ages.
The trash keeps its snapshots: it restores one entity into today's schema,
which is a different job from replaying a history.

**One gesture is one operation.** Not one per row touched. A mass status change
is a single entry naming every work it moved, because that is what the person
did and what an undo has to reverse.

**The log neither dedupes nor is swept.** `journal` (migration 0004) does both,
correctly: a feed a person reads should fold repeats and forget old news.
Applying either here would make the log replay to a database that never
existed. The two tables look alike and stay separate — deleting a work is one
line in the feed and one operation in the log, and neither can serve as the
other.

**A failed write fails the caller.** `journal::record` deliberately swallows
its errors so a full disk cannot turn a completed deletion into a reported
failure. `operation::record` returns a `Result` and callers carry it, inside
the transaction that makes the change wherever there is one. A journal with a
gap is a feed missing a line; a log with a gap replays wrongly and says nothing
about it.

A gate in `tests/` reads `commands.rs` as source and fails on a mutating
command that records no operation, the way `journal_keys.rs` reads it for
action keys. A test that merely calls the commands would pass over exactly the
command nobody remembered to call.

## Consequences

**Positive.** The database can be rebuilt from its history, which is the
property a merge, a repair and an audit all need. Intent is recorded where only
intent can be recorded, so a later merge argues about gestures rather than
guessing them from rows. The log survives schema changes that a row snapshot
would not.

**Negative.** Every mutating command grows a line, and every id- or time-minting
domain function grows a seam it did not need for its own sake. The gate keeps
the first from being forgotten; nothing but review keeps the second honest, so
a new domain function that mints an id inside itself will replay wrongly until
someone notices. The log grows without bound by design — for one person's
catalogue this is small, and it is written down here so a future profiler is
not surprised twice.

**Neutral.** Undo is not part of this decision. It becomes cheap once the log
exists, and its own stage (v0.52.0) decides what it reverses and how far back;
the owner has already settled the depth at one session. Merge semantics remain
where ADR 0012 left them.
