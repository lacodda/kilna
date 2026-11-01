-- A star on an answer worth coming back to (v0.106).
--
-- A column of the message rather than a key in its meta: the meta is the
-- proposal's, replaced whole when a proposal is marked applied, and a star
-- kept there would be one more thing a write could drop without meaning to.
-- As a column, the starred answers of every chat are one indexed read.
--
-- A message used to be written once and never changed, so its `created_at`
-- was its clock. The star is the first thing a person changes on one in
-- place, so it is clocked as a chat's title is (0011): a second device has
-- to know which of two stars came later.

ALTER TABLE chat_message ADD COLUMN starred_at TEXT;

CREATE INDEX chat_message_starred ON chat_message (starred_at)
    WHERE starred_at IS NOT NULL;

CREATE TRIGGER chat_message_clock_on_update AFTER UPDATE OF starred_at ON chat_message
BEGIN
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'chat_message', NEW.id, 'starred_at', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device)
    WHERE OLD.starred_at IS NOT NEW.starred_at
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
END;
