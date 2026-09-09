-- The operations log: what the person asked for, in the order they asked.
--
-- The schema already records what *became* of every row. Migration 0011 put a
-- tombstone under each deletion and a clock on each edited field, so a second
-- device can tell `deleted here` from `never arrived` and `this field` from
-- `something`. What none of that carries is the intent above it: SQLite sees
-- three updates where the person applied one status drift, and it sees the
-- same three whether they were one gesture or three. ADR 0012 named this gap
-- when it left the full history to "the operations log planned for the next
-- stage"; this is that log. See ADR 0014.
--
-- Three properties separate it from `journal` (0004), which looks alike and is
-- not the same thing:
--
--   * `journal` writes what a sentence needs, this writes what a replay needs;
--   * `journal` folds repeats into one line and is swept by retention, this
--     keeps every operation forever — a log with holes replays to the wrong
--     database;
--   * `journal` is read by a person in their own language, this is read by a
--     machine and carries no i18n at all.
--
-- Both stay. Deleting a work is one line in the feed and one operation here,
-- and neither can serve as the other.

CREATE TABLE operation (
    -- Ordered by `(recorded_at, seq)`: the timestamp is what two devices
    -- compare, `seq` breaks the tie inside one device. A uuid would not order
    -- anything, and two operations within one millisecond are ordinary — a
    -- mass action writes its rows as fast as SQLite accepts them.
    seq         INTEGER PRIMARY KEY AUTOINCREMENT,
    -- Stable across devices, so a merged log can tell "already have this" from
    -- "new to me" without comparing bodies.
    id          TEXT NOT NULL UNIQUE,
    -- Which workspace asked. Never null in practice; nullable so that a log
    -- written before `device` existed could still be read.
    device_id   TEXT,
    -- The profile the operation acted in, when it acted in one. Workspace-wide
    -- operations (activating a profile, restoring a backup) leave it null.
    profile_id  TEXT,
    -- What was asked, as a stable machine name: `work.create`, `works.status`.
    -- Deliberately not the journal's key: that one names a sentence and may be
    -- reworded, this one names an operation and may not.
    kind        TEXT NOT NULL,
    -- Everything the replay needs, as JSON: the person's input *and* the values
    -- the code generated while carrying it out. `work::create` mints a uuid,
    -- stamps `now()` and computes a position from the table — replaying without
    -- those would produce a different id, and every reference to it would part
    -- ways with the row it names.
    params      TEXT NOT NULL DEFAULT '{}',
    recorded_at TEXT NOT NULL
) STRICT;

-- The replay reads the whole log in order; a merge reads one device's tail.
CREATE INDEX operation_order ON operation (recorded_at, seq);
CREATE INDEX operation_device ON operation (device_id, recorded_at);
