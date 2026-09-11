-- A version's body can now change in place, for as long as nothing has judged
-- it: the editing session keeps its edits in one version rather than minting
-- one per keystroke or one per opening (ADR 0015). 0011 left `work_version`
-- without a field clock because bodies were written once and never updated;
-- a body that changes needs one, or a second device could only overwrite the
-- row whole. Same shape as the clocks in 0011, same upsert rather than
-- `OR REPLACE`, for the reason given there.

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
END;
