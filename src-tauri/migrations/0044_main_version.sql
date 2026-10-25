-- # The main version of a role (v0.100.3, ADR 0069)
--
-- A work has one current version, and since v0.99 it is always a draft: the
-- text the work is. A style prompt is not the song, so it can no longer be
-- made current - and with that went the only way to say which of six style
-- prompts is the one the song is made with. A role that is no draft gets a
-- mark of its own: the version marked main stands for the role wherever the
-- role is read - `{role:style}` in a prompt, the source of a publication -
-- and with none marked the newest one stands, as it always did.
--
-- A mark on the row rather than a pointer on the work: the roles are the
-- profile's words, not columns, and the mark goes where the row goes - to the
-- trash and back with it.
ALTER TABLE work_version ADD COLUMN is_main INTEGER NOT NULL DEFAULT 0;

-- One main version per role of a work, or none. Partial, the way 0021 keeps
-- one chosen frame per scene: the rows that are not main are not in it, so
-- they do not collide with each other.
CREATE UNIQUE INDEX work_version_one_main ON work_version (work_id, role)
WHERE is_main = 1;

-- The version's clock names its columns, so the new one joins it. Dropped and
-- written whole: SQLite has no ALTER TRIGGER (the lesson of 0012).
DROP TRIGGER work_version_clock_on_update;

CREATE TRIGGER work_version_clock_on_update AFTER UPDATE ON work_version
BEGIN
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'work_version', NEW.id, 'body', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.body IS NOT NEW.body
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'work_version', NEW.id, 'label', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.label IS NOT NEW.label
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'work_version', NEW.id, 'is_main', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.is_main IS NOT NEW.is_main
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
END;
