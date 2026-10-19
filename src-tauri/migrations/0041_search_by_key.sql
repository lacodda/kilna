-- # The search index found by key
--
-- The index (0023) is an FTS5 table, and the columns that say whose a row is -
-- `entity`, `entity_id` - are UNINDEXED: FTS5 keeps no index over them. So
-- every trigger that took a row's old text out, `DELETE ... WHERE entity = ?
-- AND entity_id = ?`, read the whole index to find it. On a workspace of
-- seven thousand rows that is 16 ms a write, unnoticed one note at a time
-- and 13 seconds for a tag renamed on 803 notes in one gesture (v0.98).
--
-- An FTS5 row is found at once by its rowid, so the rowid becomes the key:
-- `search_key` gives each indexed thing an id, and its row in the index is
-- written under that id. A trigger finds the id by the unique key and
-- deletes by rowid. The key is the index's own bookkeeping, as derived as
-- the index: rebuilt from it here, never synced, never in the trash.
CREATE TABLE search_key (
    id INTEGER PRIMARY KEY,
    entity TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    UNIQUE (entity, entity_id)
);

-- Every row the index holds takes its rowid as its key. Should a thing be
-- indexed twice, the later row keeps the key and the earlier one goes: it is
-- older text of the same thing.
INSERT INTO search_key (id, entity, entity_id)
SELECT rowid, entity, entity_id FROM search_index WHERE true ORDER BY rowid DESC
ON CONFLICT (entity, entity_id) DO NOTHING;

DELETE FROM search_index WHERE rowid NOT IN (SELECT id FROM search_key);

-- Every trigger that writes the index, written again: SQLite has no ALTER
-- TRIGGER. Each says the same as before - the text it indexes is unchanged -
-- and only finds the row by key.

DROP TRIGGER search_fact_on_delete;

CREATE TRIGGER search_fact_on_delete AFTER DELETE ON canon_fact
BEGIN
    DELETE FROM search_index WHERE rowid = (SELECT id FROM search_key WHERE entity = 'fact' AND entity_id = OLD.id);
    DELETE FROM search_key WHERE entity = 'fact' AND entity_id = OLD.id;
END;

DROP TRIGGER search_fact_on_insert;

CREATE TRIGGER search_fact_on_insert AFTER INSERT ON canon_fact
BEGIN
    INSERT INTO search_key (entity, entity_id) VALUES ('fact', NEW.id)
        ON CONFLICT (entity, entity_id) DO NOTHING;
    DELETE FROM search_index WHERE rowid = (SELECT id FROM search_key WHERE entity = 'fact' AND entity_id = NEW.id);
    INSERT INTO search_index (rowid, entity, entity_id, profile_id, work_id, body)
    VALUES ((SELECT id FROM search_key WHERE entity = 'fact' AND entity_id = NEW.id), 'fact', NEW.id, NEW.profile_id, NULL,
        NEW.body || ' ' || coalesce(NEW.when_label, '') || ' ' || coalesce(NEW.source_label, ''));
END;

DROP TRIGGER search_fact_on_update;

CREATE TRIGGER search_fact_on_update AFTER UPDATE ON canon_fact
WHEN OLD.body IS NOT NEW.body OR OLD.when_label IS NOT NEW.when_label
     OR OLD.source_label IS NOT NEW.source_label
BEGIN
    DELETE FROM search_index WHERE rowid = (SELECT id FROM search_key WHERE entity = 'fact' AND entity_id = NEW.id);
    INSERT INTO search_key (entity, entity_id) VALUES ('fact', NEW.id)
        ON CONFLICT (entity, entity_id) DO NOTHING;
    INSERT INTO search_index (rowid, entity, entity_id, profile_id, work_id, body)
    VALUES ((SELECT id FROM search_key WHERE entity = 'fact' AND entity_id = NEW.id), 'fact', NEW.id, NEW.profile_id, NULL,
        NEW.body || ' ' || coalesce(NEW.when_label, '') || ' ' || coalesce(NEW.source_label, ''));
END;

DROP TRIGGER search_message_on_delete;

CREATE TRIGGER search_message_on_delete AFTER DELETE ON chat_message
BEGIN
    DELETE FROM search_index WHERE rowid = (SELECT id FROM search_key WHERE entity = 'message' AND entity_id = OLD.id);
    DELETE FROM search_key WHERE entity = 'message' AND entity_id = OLD.id;
END;

DROP TRIGGER search_message_on_insert;

CREATE TRIGGER search_message_on_insert AFTER INSERT ON chat_message
BEGIN
    INSERT INTO search_key (entity, entity_id) VALUES ('message', NEW.id)
        ON CONFLICT (entity, entity_id) DO NOTHING;
    DELETE FROM search_index WHERE rowid = (SELECT id FROM search_key WHERE entity = 'message' AND entity_id = NEW.id);
    INSERT INTO search_index (rowid, entity, entity_id, profile_id, work_id, body)
    SELECT (SELECT id FROM search_key WHERE entity = 'message' AND entity_id = NEW.id), 'message', NEW.id, c.profile_id, c.work_id, NEW.body
      FROM chat c WHERE c.id = NEW.chat_id;
END;

DROP TRIGGER search_comment_on_delete;

CREATE TRIGGER search_comment_on_delete AFTER DELETE ON comment
BEGIN
    DELETE FROM search_index WHERE rowid = (SELECT id FROM search_key WHERE entity = 'comment' AND entity_id = OLD.id);
    DELETE FROM search_key WHERE entity = 'comment' AND entity_id = OLD.id;
END;

DROP TRIGGER search_comment_on_insert;

CREATE TRIGGER search_comment_on_insert AFTER INSERT ON comment
BEGIN
    INSERT INTO search_key (entity, entity_id) VALUES ('comment', NEW.id)
        ON CONFLICT (entity, entity_id) DO NOTHING;
    DELETE FROM search_index WHERE rowid = (SELECT id FROM search_key WHERE entity = 'comment' AND entity_id = NEW.id);
    INSERT INTO search_index (rowid, entity, entity_id, profile_id, work_id, body)
    VALUES ((SELECT id FROM search_key WHERE entity = 'comment' AND entity_id = NEW.id), 'comment', NEW.id, NEW.profile_id, NEW.work_id,
        NEW.body || ' ' || coalesce(NEW.reply, '') || ' ' ||
        coalesce(NEW.author, '') || ' ' || NEW.channel);
END;

DROP TRIGGER search_comment_on_update;

CREATE TRIGGER search_comment_on_update AFTER UPDATE ON comment
WHEN OLD.body IS NOT NEW.body OR OLD.reply IS NOT NEW.reply OR OLD.author IS NOT NEW.author
     OR OLD.channel IS NOT NEW.channel OR OLD.work_id IS NOT NEW.work_id
BEGIN
    DELETE FROM search_index WHERE rowid = (SELECT id FROM search_key WHERE entity = 'comment' AND entity_id = NEW.id);
    INSERT INTO search_key (entity, entity_id) VALUES ('comment', NEW.id)
        ON CONFLICT (entity, entity_id) DO NOTHING;
    INSERT INTO search_index (rowid, entity, entity_id, profile_id, work_id, body)
    VALUES ((SELECT id FROM search_key WHERE entity = 'comment' AND entity_id = NEW.id), 'comment', NEW.id, NEW.profile_id, NEW.work_id,
        NEW.body || ' ' || coalesce(NEW.reply, '') || ' ' ||
        coalesce(NEW.author, '') || ' ' || NEW.channel);
END;

DROP TRIGGER search_note_on_delete;

CREATE TRIGGER search_note_on_delete AFTER DELETE ON note
BEGIN
    DELETE FROM search_index WHERE rowid = (SELECT id FROM search_key WHERE entity = 'note' AND entity_id = OLD.id);
    DELETE FROM search_key WHERE entity = 'note' AND entity_id = OLD.id;
END;

DROP TRIGGER search_note_on_insert;

CREATE TRIGGER search_note_on_insert AFTER INSERT ON note
BEGIN
    INSERT INTO search_key (entity, entity_id) VALUES ('note', NEW.id)
        ON CONFLICT (entity, entity_id) DO NOTHING;
    DELETE FROM search_index WHERE rowid = (SELECT id FROM search_key WHERE entity = 'note' AND entity_id = NEW.id);
    INSERT INTO search_index (rowid, entity, entity_id, profile_id, work_id, body)
    VALUES ((SELECT id FROM search_key WHERE entity = 'note' AND entity_id = NEW.id), 'note', NEW.id, NEW.profile_id, NEW.work_id,
        coalesce(NEW.title, '') || ' ' || NEW.body || ' ' || coalesce(NEW.attribution, '') || ' ' ||
        coalesce((SELECT group_concat(value, ' ') FROM json_each(NEW.tags)), '') || ' ' ||
        coalesce((SELECT group_concat(value, ' ') FROM json_each(NEW.aliases)), ''));
END;

DROP TRIGGER search_note_on_update;

CREATE TRIGGER search_note_on_update AFTER UPDATE ON note
WHEN OLD.title IS NOT NEW.title OR OLD.body IS NOT NEW.body OR OLD.tags IS NOT NEW.tags
     OR OLD.aliases IS NOT NEW.aliases OR OLD.work_id IS NOT NEW.work_id
     OR OLD.attribution IS NOT NEW.attribution
BEGIN
    DELETE FROM search_index WHERE rowid = (SELECT id FROM search_key WHERE entity = 'note' AND entity_id = NEW.id);
    INSERT INTO search_key (entity, entity_id) VALUES ('note', NEW.id)
        ON CONFLICT (entity, entity_id) DO NOTHING;
    INSERT INTO search_index (rowid, entity, entity_id, profile_id, work_id, body)
    VALUES ((SELECT id FROM search_key WHERE entity = 'note' AND entity_id = NEW.id), 'note', NEW.id, NEW.profile_id, NEW.work_id,
        coalesce(NEW.title, '') || ' ' || NEW.body || ' ' || coalesce(NEW.attribution, '') || ' ' ||
        coalesce((SELECT group_concat(value, ' ') FROM json_each(NEW.tags)), '') || ' ' ||
        coalesce((SELECT group_concat(value, ' ') FROM json_each(NEW.aliases)), ''));
END;

DROP TRIGGER search_work_on_delete;

CREATE TRIGGER search_work_on_delete AFTER DELETE ON work
BEGIN
    DELETE FROM search_index WHERE rowid = (SELECT id FROM search_key WHERE entity = 'work' AND entity_id = OLD.id);
    DELETE FROM search_key WHERE entity = 'work' AND entity_id = OLD.id;
END;

DROP TRIGGER search_work_on_insert;

CREATE TRIGGER search_work_on_insert AFTER INSERT ON work
BEGIN
    INSERT INTO search_key (entity, entity_id) VALUES ('work', NEW.id)
        ON CONFLICT (entity, entity_id) DO NOTHING;
    DELETE FROM search_index WHERE rowid = (SELECT id FROM search_key WHERE entity = 'work' AND entity_id = NEW.id);
    INSERT INTO search_index (rowid, entity, entity_id, profile_id, work_id, body)
    VALUES ((SELECT id FROM search_key WHERE entity = 'work' AND entity_id = NEW.id), 'work', NEW.id, NEW.profile_id, NEW.id,
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
    DELETE FROM search_index WHERE rowid = (SELECT id FROM search_key WHERE entity = 'work' AND entity_id = NEW.id);
    INSERT INTO search_key (entity, entity_id) VALUES ('work', NEW.id)
        ON CONFLICT (entity, entity_id) DO NOTHING;
    INSERT INTO search_index (rowid, entity, entity_id, profile_id, work_id, body)
    VALUES ((SELECT id FROM search_key WHERE entity = 'work' AND entity_id = NEW.id), 'work', NEW.id, NEW.profile_id, NEW.id,
        NEW.title || ' ' ||
        coalesce((SELECT group_concat(value, ' ') FROM json_each(NEW.meta)
                   WHERE type IN ('text', 'integer', 'real')), '') || ' ' ||
        coalesce((SELECT group_concat(value, ' ') FROM json_each(NEW.tags)), '') || ' ' ||
        coalesce((SELECT group_concat(value, ' ') FROM json_each(NEW.cover)
                   WHERE type = 'text'), '') || ' ' ||
        coalesce((SELECT group_concat(value, ' ') FROM json_each(NEW.frame)
                   WHERE type = 'text'), ''));
END;

DROP TRIGGER search_version_on_delete;

CREATE TRIGGER search_version_on_delete AFTER DELETE ON work_version
BEGIN
    DELETE FROM search_index WHERE rowid = (SELECT id FROM search_key WHERE entity = 'version' AND entity_id = OLD.id);
    DELETE FROM search_key WHERE entity = 'version' AND entity_id = OLD.id;
END;

DROP TRIGGER search_version_on_insert;

CREATE TRIGGER search_version_on_insert AFTER INSERT ON work_version
BEGIN
    INSERT INTO search_key (entity, entity_id) VALUES ('version', NEW.id)
        ON CONFLICT (entity, entity_id) DO NOTHING;
    DELETE FROM search_index WHERE rowid = (SELECT id FROM search_key WHERE entity = 'version' AND entity_id = NEW.id);
    INSERT INTO search_index (rowid, entity, entity_id, profile_id, work_id, body)
    SELECT (SELECT id FROM search_key WHERE entity = 'version' AND entity_id = NEW.id), 'version', NEW.id, w.profile_id, NEW.work_id, NEW.body
      FROM work w WHERE w.id = NEW.work_id;
END;

DROP TRIGGER search_version_on_update;

CREATE TRIGGER search_version_on_update AFTER UPDATE ON work_version
WHEN OLD.body IS NOT NEW.body
BEGIN
    DELETE FROM search_index WHERE rowid = (SELECT id FROM search_key WHERE entity = 'version' AND entity_id = NEW.id);
    INSERT INTO search_key (entity, entity_id) VALUES ('version', NEW.id)
        ON CONFLICT (entity, entity_id) DO NOTHING;
    INSERT INTO search_index (rowid, entity, entity_id, profile_id, work_id, body)
    SELECT (SELECT id FROM search_key WHERE entity = 'version' AND entity_id = NEW.id), 'version', NEW.id, w.profile_id, NEW.work_id, NEW.body
      FROM work w WHERE w.id = NEW.work_id;
END;
