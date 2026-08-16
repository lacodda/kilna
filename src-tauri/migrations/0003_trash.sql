-- The trash: nothing is lost by pressing a button.
--
-- A deletion moves rows out of the live tables and into a snapshot here, rather
-- than marking them dead in place. The live tables stay exactly as they were, so
-- no existing query has to remember a `deleted_at IS NULL` clause — a filter
-- nobody can forget is worth more than one that is merely easy to add.
CREATE TABLE deletion (
    id         TEXT PRIMARY KEY,
    -- Which profile the entry belongs to, so the trash is scoped like everything
    -- else. Kept as a plain value rather than a reference: dropping a profile
    -- must not silently empty the trash it left behind.
    profile_id TEXT,
    -- The kind of thing deleted: work, version, score, release, note.
    entity     TEXT NOT NULL,
    -- Id the row had while it was alive. Restoring puts it back under this id,
    -- so anything that still points at it keeps pointing at it.
    entity_id  TEXT NOT NULL,
    -- What it was, in one line, for the trash screen to show without unpacking
    -- the snapshot: a work's title, a note's first words.
    label      TEXT NOT NULL,
    -- Where it came from, for the same reason: the parent work's title, so an
    -- orphan version is recognisable.
    origin     TEXT,
    -- Why it went: 'manual' for a person pressing delete, or the entity that
    -- took it down with it when a parent was deleted.
    reason     TEXT NOT NULL DEFAULT 'manual',
    -- The rows themselves, keyed by table name:
    -- {"work": [{...}], "work_version": [{...}, ...]}. Column names and values
    -- exactly as they were, so a restore is an insert, not a reconstruction.
    snapshot   TEXT NOT NULL,
    deleted_at TEXT NOT NULL
) STRICT;

CREATE INDEX deletion_profile ON deletion (profile_id, deleted_at DESC);
CREATE INDEX deletion_entity ON deletion (entity, entity_id);
