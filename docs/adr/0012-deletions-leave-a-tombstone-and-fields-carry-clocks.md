# 0012 — Deletions leave a tombstone and fields carry clocks

Date: 2026-09-07
Status: Accepted

## Context

ADR 0002 makes kilna local-first: one SQLite file is the workspace. The
trash (migration 0003) then made deletion safe on that one machine by moving
rows out of the live tables into a snapshot, so that no query anywhere has to
remember a `deleted_at IS NULL` clause. That reasoning still holds and this
decision does not touch it.

What it does not cover is a second copy of the workspace. The research of
2026-08-26 on a mobile companion found the gap: once a row is physically
gone, *deleted here* reads exactly like *never arrived here*. Any merge —
a legacy import today, a second device later — brings the row back. The
trash cannot serve as the record, because the trash is a drawer the person
is meant to empty, and because two kinds of deletion never visit it at all:
a board note and a chat are dropped outright by design (a drawer of
crossed-out reminders is not something anyone opens), and everything a work
takes down with it goes through the schema's `ON DELETE CASCADE`, not
through the trash code.

The same research named the second half. Six tables are edited in place —
`work`, `release`, `note`, `collection`, `profile`, `chat`; the focus board
added a seventh since — and each row says only `updated_at`: that
*something* changed, not what. Two devices that edited different fields of
one row could only ever overwrite each other whole. A merge that keeps both
edits needs a clock per field, and a tie-break both sides compute the same
way when two clocks agree to the millisecond.

Both are changes to what the schema records, which is why they come now, in
the block of breaking changes before 1.0 freezes the format, rather than the
day a merge is written.

## Decision

**Every deletion leaves a tombstone, every in-place edit stamps the field,
and the schema does both itself.**

Migration 0011 adds three tables and a set of triggers:

- `device` — one row, the identity of this workspace, minted the first time
  a build that knows about it opens the file and never changed. It is the
  deterministic tie-break: when two devices' clocks agree, the device id
  settles the order the same way everywhere.
- `tombstone (entity, entity_id, profile_id, label, deleted_at, deleted_by,
  restored_at, restored_by)` — keyed by table and id. Written by an
  `AFTER DELETE` trigger on every table that holds a fact about the person's
  work: `profile`, `collection`, `work`, `work_version`, `work_score`,
  `release`, `note`, `asset`, `chat`, `chat_message`, `focus_note`,
  `focus_dismissal`. An `AFTER INSERT` trigger on the same tables stamps
  `restored_at` when a row with a buried id comes back, so a restore from the
  trash — an insert of the same id — is recorded without the trash knowing.
  Deleting the id again replaces the entry and clears the restore: the newest
  event says whether the row is alive.
- `field_clock (entity, entity_id, field, changed_at, device_id)` — written by
  an `AFTER UPDATE` trigger on each in-place table, one row per column whose
  value actually changed (`OLD.x IS NOT NEW.x`, so a move to or from NULL
  counts and a save of the same values leaves nothing). A row's clocks are
  dropped with the row; a restored row starts clean, and `restored_at` is the
  moment it counts as alive again.

Triggers rather than code, for the reason 0003 chose a snapshot table over a
column: a rule the schema enforces is one no code path can forget. The
cascade beneath a deleted work, a board note dropped by the board, a chat
closed by the panel, a collection letting go of its works through
`ON DELETE SET NULL` — each reaches the right table through the schema and
is caught there. Timestamps inside triggers take the application's own
shape, `2026-09-07T10:11:12.345Z`, so the two kinds of stamp sort together.

Columns that describe this machine rather than the work are deliberately not
clocked: a profile's `is_active`, a chat's CLI `session_id` and
`waiting_since`. They must not travel. `journal` (append-only, trimmed by
retention), `chat_run` (the live state of a process here) and `deletion`
(the trash itself) get no tombstones for the same reason.

The layout of the calendar is part of the same promise. `layout::plan` was
already a pure function of the database, so after a merge the calendar is
recomputed rather than merged — but a pure function of rows that tie is a
function of the order they were written in, which two devices do not share.
The plan now breaks its last tie on the release id: not an order anyone
reads, but one every device computes alike. It lives in `plan` and not in the
SQL because the ordering gate is right that a uuid is not an order for a
person; it is only a total order for a machine.

One consumer exists today. The legacy import, which knows nothing but
titles, consults the work tombstones of the profile and leaves a title the
person deleted where it is — reported as such rather than silently — even
after the trash has been emptied.

## Consequences

**Positive.** The schema can tell *deleted* from *never here*, and *which
field changed* from *something changed*, for every row written from this
version on. A future merge — two workspaces on a table, a relay, a phone —
has the facts it needs without a second migration of everything already
stored. The guarantee is structural: adding a delete path or an edit path to
the code cannot lose it.

**Negative.** Every update on an in-place table costs an extra write per
changed column, and every delete an extra row. For a single person's
catalogue the cost is invisible; it is noted here so a future profiler is
not surprised. Deleting the same id many times keeps only the latest cycle —
the operations log planned for the next stage carries the full history, and
this table is not asked to.

**Neutral.** A restored backup is *this device, continued*: the id travels
with the file, which is the right reading of a restore. Copying a workspace
to make a second device is exactly the operation the future merge will have
to give its own name to. Merge semantics themselves — which of two edits to
the same field wins, what a deletion does to a row edited since — are not
decided here; the research of 2026-08-26 sketches them and the stage that
writes the merge will decide.
