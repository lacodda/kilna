-- The frame of a work that plays under one picture for its whole length (v0.86).
--
-- An audio release on a video platform is a track with a picture on it: one
-- still that stays on screen for the whole song, and a short loop of what
-- moves in it - a leaf drifting, dust in a beam - repeated end to end. The
-- still and what to keep out of it are text a person writes; the loop is
-- written from settings: how long one turn runs, whether the camera holds
-- still, whether the last frame meets the first (ADR 0046).
--
-- Unlike the cover's parts these are the application's, not the craft's, so
-- the JSON has a fixed shape - {still, motion, seconds, still_camera,
-- seamless, negative} - read into one struct. It is a column on `work` for
-- the cover's reason: one frame per work, a body rather than a field.
ALTER TABLE work ADD COLUMN frame TEXT NOT NULL DEFAULT '{}';

-- The clock trigger names its columns one by one (the lesson of 0023), so it
-- is rewritten whole to take the new column in.
DROP TRIGGER work_clock_on_update;
CREATE TRIGGER work_clock_on_update AFTER UPDATE ON work
BEGIN
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'work', NEW.id, 'collection_id', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.collection_id IS NOT NEW.collection_id
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'work', NEW.id, 'kind', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.kind IS NOT NEW.kind
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'work', NEW.id, 'title', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.title IS NOT NEW.title
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'work', NEW.id, 'status', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.status IS NOT NEW.status
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'work', NEW.id, 'meta', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.meta IS NOT NEW.meta
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'work', NEW.id, 'current_version_id', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.current_version_id IS NOT NEW.current_version_id
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'work', NEW.id, 'position', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.position IS NOT NEW.position
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'work', NEW.id, 'status_pinned_at', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.status_pinned_at IS NOT NEW.status_pinned_at
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'work', NEW.id, 'tags', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.tags IS NOT NEW.tags
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'work', NEW.id, 'marks', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.marks IS NOT NEW.marks
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'work', NEW.id, 'tier_pinned', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.tier_pinned IS NOT NEW.tier_pinned
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'work', NEW.id, 'tier_pinned_at', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.tier_pinned_at IS NOT NEW.tier_pinned_at
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'work', NEW.id, 'tier_pin_reason', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.tier_pin_reason IS NOT NEW.tier_pin_reason
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'work', NEW.id, 'bookmarked_at', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.bookmarked_at IS NOT NEW.bookmarked_at
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'work', NEW.id, 'stage', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.stage IS NOT NEW.stage
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'work', NEW.id, 'cover', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.cover IS NOT NEW.cover
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'work', NEW.id, 'frame', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.frame IS NOT NEW.frame
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
END;

-- The frame is text about the work, so the search finds it the way it finds
-- the cover's prompt.
DROP TRIGGER search_work_on_insert;
CREATE TRIGGER search_work_on_insert AFTER INSERT ON work
BEGIN
    INSERT INTO search_index (entity, entity_id, profile_id, work_id, body)
    VALUES ('work', NEW.id, NEW.profile_id, NEW.id,
        NEW.title || ' ' ||
        coalesce((SELECT group_concat(value, ' ') FROM json_each(NEW.meta)
                   WHERE type IN ('text', 'integer', 'real')), '') || ' ' ||
        coalesce((SELECT group_concat(value, ' ') FROM json_each(NEW.tags)), '') || ' ' ||
        coalesce((SELECT group_concat(value, ' ') FROM json_each(NEW.cover)
                   WHERE type = 'text'), '') || ' ' ||
        coalesce((SELECT group_concat(value, ' ') FROM json_each(NEW.frame)
                   WHERE type = 'text'), ''));
END;

DROP TRIGGER search_work_on_update;
CREATE TRIGGER search_work_on_update AFTER UPDATE ON work
WHEN OLD.title IS NOT NEW.title OR OLD.meta IS NOT NEW.meta OR OLD.tags IS NOT NEW.tags
     OR OLD.cover IS NOT NEW.cover OR OLD.frame IS NOT NEW.frame
BEGIN
    DELETE FROM search_index WHERE entity = 'work' AND entity_id = NEW.id;
    INSERT INTO search_index (entity, entity_id, profile_id, work_id, body)
    VALUES ('work', NEW.id, NEW.profile_id, NEW.id,
        NEW.title || ' ' ||
        coalesce((SELECT group_concat(value, ' ') FROM json_each(NEW.meta)
                   WHERE type IN ('text', 'integer', 'real')), '') || ' ' ||
        coalesce((SELECT group_concat(value, ' ') FROM json_each(NEW.tags)), '') || ' ' ||
        coalesce((SELECT group_concat(value, ' ') FROM json_each(NEW.cover)
                   WHERE type = 'text'), '') || ' ' ||
        coalesce((SELECT group_concat(value, ' ') FROM json_each(NEW.frame)
                   WHERE type = 'text'), ''));
END;
