-- An asset knows whose it is, and what the world calls it.
--
-- The table has stood since the first migration with no writer (ADR 0008),
-- shaped for a cover on a work or a release. Giving it a writer shows two
-- gaps.
--
-- It has no `profile_id`. Every other table a person's work lives in carries
-- one, and the tombstone trigger of 0011 had to write NULL into a column it
-- could not read — so a deleted asset came back in no workspace at all. A
-- profile is also what makes "the files of this workspace" a question with an
-- answer.
--
-- And it has no memory of the name the file arrived under. A file is copied
-- into the workspace's own `media/` directory and named by its id there, so
-- nothing outside can rename it and nothing inside collides; but the name it
-- had is what the person recognises it by, and what a generator wrote in it
-- ("scene-04-still-v3.png"). That name is kept, and shown.
--
-- Both are added rather than rebuilt: the table is empty in every workspace
-- alive, so there is nothing to migrate and nothing to lose.

ALTER TABLE asset ADD COLUMN profile_id TEXT REFERENCES profile (id) ON DELETE CASCADE;
ALTER TABLE asset ADD COLUMN original_name TEXT;

CREATE INDEX asset_profile ON asset (profile_id);

-- The tombstone can now say whose the asset was. Dropped and rewritten
-- rather than edited: SQLite has no ALTER TRIGGER.
DROP TRIGGER asset_tombstone_on_delete;

CREATE TRIGGER asset_tombstone_on_delete AFTER DELETE ON asset
BEGIN
    INSERT INTO tombstone (entity, entity_id, profile_id, label, deleted_at, deleted_by)
    VALUES ('asset', OLD.id, OLD.profile_id, OLD.label, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device))
    ON CONFLICT (entity, entity_id) DO UPDATE SET
        profile_id = excluded.profile_id, label = excluded.label,
        deleted_at = excluded.deleted_at, deleted_by = excluded.deleted_by,
        restored_at = NULL, restored_by = NULL;
    DELETE FROM field_clock WHERE entity = 'asset' AND entity_id = OLD.id;
END;
