-- A scene's built frame (v0.88).
--
-- The cover constructor builds a picture's frame from a layout and five
-- settings - where the hero stands, how big, how much of them shows - and
-- writes the prompt from them (ADR 0049). A scene of a clip is the same
-- picture without words, so it takes the same frame: when a scene has one,
-- its still is written around the block that says what the picture shows,
-- in the clip's style. Absent, the scene's blocks are copied as written,
-- as they were before.
--
-- A JSON object of the application's fixed shape, read into one struct; a
-- column on `scene` for the reason the blocks are one: a scene owns it.
ALTER TABLE scene ADD COLUMN framing TEXT;

-- The clock trigger names its columns one by one (the lesson of 0023), so it
-- is rewritten whole to take the new column in.
DROP TRIGGER scene_clock_on_update;
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
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'scene', NEW.id, 'framing', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.framing IS NOT NEW.framing
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
END;
