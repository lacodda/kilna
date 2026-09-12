-- A scene: one row of a video's storyboard.
--
-- A scene belongs to the work — not to a release and not to a version
-- (decision of 2026-09-11). A second attempt at a video is a second video
-- made from the same donor, and the scenes go with the work they are the
-- storyboard of. The predecessor kept scenes as markdown files edited whole,
-- and "700 of 2727 files were one save away from corruption": here a scene
-- is a row with fields, and the text a person edits is one field or one
-- prompt block at a time.
--
-- The number is a plain integer with no UNIQUE on it: renumbering with a
-- shift is its own stage (v0.67), and a scene restored from the trash must
-- not be refused because its number was taken meanwhile. Order is by number,
-- then by insertion.
--
-- The kind of shot is a key of the kind's `shot_types` vocabulary, so the
-- board can be narrowed to "every detail". The prompt blocks are a JSON
-- object keyed by the kind's `scene_blocks` — a still frame, an animation, a
-- negative — each edited on its own and copied on its own.

CREATE TABLE scene (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profile (id) ON DELETE CASCADE,
    work_id TEXT NOT NULL REFERENCES work (id) ON DELETE CASCADE,
    -- The scene's number on the board, from 1.
    position INTEGER NOT NULL,
    -- The part of the text it plays against: intro, verse 1, chorus. Free
    -- text until the text's own markup gives the board its frame (v0.61).
    section TEXT,
    -- Seconds from the start of the video; unset until the board is timed.
    starts_at REAL,
    ends_at REAL,
    -- A key of the kind's `shot_types`; unset when not yet decided.
    shot_type TEXT,
    description TEXT NOT NULL DEFAULT '',
    -- Prompt blocks by the kind's `scene_blocks` key: {"still": "...", ...}.
    blocks TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX scene_work ON scene (work_id, position);

-- Tombstones and field clocks, the shape 0011 gives every table a person's
-- work lives in.
CREATE TRIGGER scene_tombstone_on_delete AFTER DELETE ON scene
BEGIN
    INSERT INTO tombstone (entity, entity_id, profile_id, label, deleted_at, deleted_by)
    VALUES ('scene', OLD.id, OLD.profile_id, 'scene ' || OLD.position, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device))
    ON CONFLICT (entity, entity_id) DO UPDATE SET
        profile_id = excluded.profile_id, label = excluded.label,
        deleted_at = excluded.deleted_at, deleted_by = excluded.deleted_by,
        restored_at = NULL, restored_by = NULL;
    DELETE FROM field_clock WHERE entity = 'scene' AND entity_id = OLD.id;
END;

CREATE TRIGGER scene_tombstone_on_insert AFTER INSERT ON scene
BEGIN
    UPDATE tombstone SET restored_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), restored_by = (SELECT id FROM device)
    WHERE entity = 'scene' AND entity_id = NEW.id AND restored_at IS NULL;
END;

CREATE TRIGGER scene_clock_on_update AFTER UPDATE ON scene
BEGIN
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'scene', NEW.id, 'position', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.position IS NOT NEW.position
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'scene', NEW.id, 'section', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.section IS NOT NEW.section
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'scene', NEW.id, 'starts_at', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.starts_at IS NOT NEW.starts_at
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'scene', NEW.id, 'ends_at', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.ends_at IS NOT NEW.ends_at
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'scene', NEW.id, 'shot_type', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.shot_type IS NOT NEW.shot_type
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'scene', NEW.id, 'description', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.description IS NOT NEW.description
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'scene', NEW.id, 'blocks', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.blocks IS NOT NEW.blocks
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
END;
