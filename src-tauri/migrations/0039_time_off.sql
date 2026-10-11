-- Days the calendar is closed (v0.96).
--
-- A holiday, a fortnight away, the week a move takes: days nothing should go
-- out on, said once, for the auto-layout to step around and for the month to
-- show. A row is a span of days with a name - one day or forty - because that
-- is how a person says it: "away from the 20th to the 27th", "New Year".
--
-- ## Why a span, and not a mark on a day
--
-- A mark per day would make a fortnight fourteen rows to add, fourteen to
-- take back, and fourteen to name alike; the span is the fact the person
-- states. Both ends are days (`yyyy-mm-dd`) and both are inside it, so a
-- single day is a span whose ends meet.
--
-- ## What it closes, and what it does not
--
-- It closes the days to the auto-layout: a plan never lands on them. It does
-- not close them to a person. A release dropped on a closed day lands, with
-- the closure said before it does - the footing a kept date and a repeated
-- word already have. A rule that argued with a deliberate gesture is what the
-- calendar gave up in v0.44.
--
-- It always has a name - "Away", "New Year" - because the month shows the
-- name on the days it closes, and a closed day that cannot say why reads as
-- a fault. The window offers a plain word to keep.
--
-- It belongs to the profile, as the calendar and its rhythm do.

CREATE TABLE time_off (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profile (id) ON DELETE CASCADE,
    starts_on TEXT NOT NULL,
    ends_on TEXT NOT NULL,
    label TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    -- Days, compared as text: the shape is what makes the comparison true.
    CHECK (length(starts_on) = 10 AND starts_on GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
    CHECK (length(ends_on) = 10 AND ends_on GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
    CHECK (starts_on <= ends_on),
    CHECK (length(trim(label)) > 0)
);

CREATE INDEX time_off_profile ON time_off (profile_id, starts_on);

-- Tombstones, the shape 0011 gives every table a person's work lives in. The
-- label is the name.
CREATE TRIGGER time_off_tombstone_on_delete AFTER DELETE ON time_off
BEGIN
    INSERT INTO tombstone (entity, entity_id, profile_id, label, deleted_at, deleted_by)
    VALUES ('time_off', OLD.id, OLD.profile_id,
        OLD.label,
        strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device))
    ON CONFLICT (entity, entity_id) DO UPDATE SET
        profile_id = excluded.profile_id, label = excluded.label,
        deleted_at = excluded.deleted_at, deleted_by = excluded.deleted_by,
        restored_at = NULL, restored_by = NULL;
    DELETE FROM field_clock WHERE entity = 'time_off' AND entity_id = OLD.id;
END;

CREATE TRIGGER time_off_tombstone_on_insert AFTER INSERT ON time_off
BEGIN
    UPDATE tombstone SET restored_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), restored_by = (SELECT id FROM device)
    WHERE entity = 'time_off' AND entity_id = NEW.id AND restored_at IS NULL;
END;

-- Field clocks, so a span moved on one machine and renamed on another settles
-- the way every other edit settles (0011). One statement per column: a column
-- left out here is invisible to synchronisation (the lesson of 0023).
CREATE TRIGGER time_off_clock_on_update AFTER UPDATE ON time_off
BEGIN
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'time_off', NEW.id, 'starts_on', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.starts_on IS NOT NEW.starts_on
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'time_off', NEW.id, 'ends_on', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.ends_on IS NOT NEW.ends_on
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'time_off', NEW.id, 'label', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.label IS NOT NEW.label
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
END;
