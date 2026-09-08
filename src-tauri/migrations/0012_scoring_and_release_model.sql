-- The model of scoring and release: the second step of the model package.
--
-- Everything here is a fact about one row that the schema could not hold
-- before — who judged, what tier a person is holding a work at and why, which
-- draft a draft was written from, when in the day a release goes out, what a
-- collection is aiming for. Rules of the craft (axis kinds, per-kind weights,
-- catalogue columns) live in the profile document, not here — see ADR 0013.
--
-- Every column is optional with no default meaning changed: a workspace
-- written before this migration reads exactly as it did.

-- Who gave the score. NULL is the author; a name is a second opinion brought
-- in beside their own.
ALTER TABLE work_score ADD COLUMN rater TEXT;

-- A tier held by hand, with the reason. The pattern of the status pin (0005):
-- NULL means the automation's verdict stands, a key means a person overruled
-- it and `tier_pin_reason` says why — a pin without a reason is a number
-- nobody can argue with later.
ALTER TABLE work ADD COLUMN tier_pinned TEXT;
ALTER TABLE work ADD COLUMN tier_pinned_at TEXT;
ALTER TABLE work ADD COLUMN tier_pin_reason TEXT;

-- A bookmark: "come back to this one". Not a status (it derives nothing), not
-- a mark (it is not a word from the profile), not a pin (that word is taken
-- twice already). A timestamp so the bookmarked can be read in the order they
-- were set.
ALTER TABLE work ADD COLUMN bookmarked_at TEXT;

-- When in the day, and in whose day. The slot stays a date — the calendar is
-- read a month at a time — and the time sits beside it for the platforms that
-- ask for one. `time_zone` is an IANA name, so the pair still means one
-- instant when read on another machine.
ALTER TABLE release ADD COLUMN scheduled_time TEXT;
ALTER TABLE release ADD COLUMN time_zone TEXT;

-- The draft this draft was written from. Revisions stay a line per role for
-- numbering; this is the tree behind the line. Set NULL when the parent goes,
-- so a pruned branch keeps its leaves.
ALTER TABLE work_version ADD COLUMN parent_version_id TEXT REFERENCES work_version (id) ON DELETE SET NULL;

-- What a collection is aiming for: how many works, and by when (a date).
ALTER TABLE collection ADD COLUMN target_size INTEGER;
ALTER TABLE collection ADD COLUMN due_on TEXT;

-- A board note with a date: "by Friday". A date, not an instant.
ALTER TABLE focus_note ADD COLUMN due_on TEXT;

-- ---- Field clocks, extended -------------------------------------------------
--
-- The update triggers of 0011 name their columns, so a new column on a clocked
-- table is silent until its trigger is rewritten. Dropped and recreated whole
-- rather than patched, so the trigger reads as one list.

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
END;

DROP TRIGGER release_clock_on_update;
CREATE TRIGGER release_clock_on_update AFTER UPDATE ON release
BEGIN
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'release', NEW.id, 'kind', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.kind IS NOT NEW.kind
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'release', NEW.id, 'status', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.status IS NOT NEW.status
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'release', NEW.id, 'title', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.title IS NOT NEW.title
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'release', NEW.id, 'scheduled_at', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.scheduled_at IS NOT NEW.scheduled_at
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'release', NEW.id, 'released_at', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.released_at IS NOT NEW.released_at
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'release', NEW.id, 'url', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.url IS NOT NEW.url
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'release', NEW.id, 'meta', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.meta IS NOT NEW.meta
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'release', NEW.id, 'slot_pinned_at', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.slot_pinned_at IS NOT NEW.slot_pinned_at
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'release', NEW.id, 'scheduled_time', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.scheduled_time IS NOT NEW.scheduled_time
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'release', NEW.id, 'time_zone', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.time_zone IS NOT NEW.time_zone
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
END;

DROP TRIGGER collection_clock_on_update;
CREATE TRIGGER collection_clock_on_update AFTER UPDATE ON collection
BEGIN
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'collection', NEW.id, 'kind', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.kind IS NOT NEW.kind
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'collection', NEW.id, 'title', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.title IS NOT NEW.title
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'collection', NEW.id, 'description', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.description IS NOT NEW.description
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'collection', NEW.id, 'position', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.position IS NOT NEW.position
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'collection', NEW.id, 'meta', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.meta IS NOT NEW.meta
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'collection', NEW.id, 'target_size', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.target_size IS NOT NEW.target_size
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'collection', NEW.id, 'due_on', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.due_on IS NOT NEW.due_on
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
END;

DROP TRIGGER focus_note_clock_on_update;
CREATE TRIGGER focus_note_clock_on_update AFTER UPDATE ON focus_note
BEGIN
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'focus_note', NEW.id, 'body', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.body IS NOT NEW.body
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'focus_note', NEW.id, 'work_id', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.work_id IS NOT NEW.work_id
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'focus_note', NEW.id, 'position', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.position IS NOT NEW.position
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'focus_note', NEW.id, 'pinned_at', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.pinned_at IS NOT NEW.pinned_at
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'focus_note', NEW.id, 'due_on', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.due_on IS NOT NEW.due_on
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
END;
