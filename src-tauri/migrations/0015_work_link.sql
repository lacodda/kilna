-- A work made from another: a video from a song, a video from an article, a
-- video from several sources. One table for every role a link can have
-- (decision of 2026-09-11): `donor` now, remix and sequel later on the same
-- rows, so the second role is a value rather than a migration.
--
-- The link remembers which version of the source it was taken at. Without it
-- "the source changed since" cannot be told from "the source is as it was",
-- and the card could only guess; with it the card can state the fact and
-- point at the diff, and leave the conclusion to the person — the same rule
-- a stale score follows.
--
-- Both sides cascade: a link is about two works and means nothing without
-- either. The trash keeps a copy under the work it took down, from either
-- side, so a restore brings the link back with the work.

CREATE TABLE work_link (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profile (id) ON DELETE CASCADE,
    -- The work that was made from the other.
    work_id TEXT NOT NULL REFERENCES work (id) ON DELETE CASCADE,
    -- The work it was made from.
    source_id TEXT NOT NULL REFERENCES work (id) ON DELETE CASCADE,
    role TEXT NOT NULL,
    -- The source's current version at the moment the link was made. Set to
    -- nothing when that version is deleted: the link stays, the fact of which
    -- version it was is gone with the version.
    source_version_id TEXT REFERENCES work_version (id) ON DELETE SET NULL,
    created_at TEXT NOT NULL,
    UNIQUE (work_id, source_id, role),
    CHECK (work_id <> source_id)
);

CREATE INDEX work_link_source ON work_link (source_id);

-- Tombstones, the same shape as 0011 gives every table a person's work lives
-- in: a link deleted here must not read as a link that never arrived here.
CREATE TRIGGER work_link_tombstone_on_delete AFTER DELETE ON work_link
BEGIN
    INSERT INTO tombstone (entity, entity_id, profile_id, label, deleted_at, deleted_by)
    VALUES ('work_link', OLD.id, OLD.profile_id, OLD.role, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device))
    ON CONFLICT (entity, entity_id) DO UPDATE SET
        profile_id = excluded.profile_id, label = excluded.label,
        deleted_at = excluded.deleted_at, deleted_by = excluded.deleted_by,
        restored_at = NULL, restored_by = NULL;
END;

CREATE TRIGGER work_link_tombstone_on_insert AFTER INSERT ON work_link
BEGIN
    UPDATE tombstone SET restored_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), restored_by = (SELECT id FROM device)
    WHERE entity = 'work_link' AND entity_id = NEW.id AND restored_at IS NULL;
END;
