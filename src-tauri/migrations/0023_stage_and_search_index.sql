-- Two things a catalogue was missing: how far along a work is, and a way to
-- find one by what it says rather than by what it is called.
--
-- ## The stage
--
-- How finished the work is, as a percentage. Not a status: a status here is
-- derived from facts (a score exists, a release is booked, it shipped), and
-- how polished a draft feels is a judgement nothing can derive. Not a mark
-- either - a mark is on or off, and the question is one of degree.
--
-- The number is the storage and the profile's `stages` name its stops, rather
-- than a key here and a percentage in the interface: a dial is a fraction by
-- nature, and a second table mapping key to fraction would be a second truth
-- about the same thing.
--
-- NULL is a third state, not a zero: a work nobody has judged yet is not the
-- same as a work judged to be a bare idea, and every work that exists today is
-- the former.
ALTER TABLE work ADD COLUMN stage INTEGER;

-- The clock triggers of 0011 name their columns one by one, so a new column on
-- a clocked table is invisible to synchronisation until the trigger is
-- rewritten. Dropped and recreated whole, so the list reads as one.

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
END;

-- ## The search index
--
-- Until now the palette read every version body of the profile into memory on
-- each keystroke and folded it in Rust, because SQLite's `LIKE` folds ASCII
-- only and half of this workspace is Russian. FTS5's `unicode61` tokenizer
-- folds the whole of Unicode, which removes the reason that loop existed.
--
-- Not `content=`: an external-content index mirrors exactly one table, and a
-- hit here can come from a work's title, its craft fields, its tags, a version
-- body or a note. The text is assembled by the triggers below and stored, so
-- one query answers for all of them.
--
-- `entity`, `entity_id`, `profile_id` and `work_id` are UNINDEXED: they are
-- carried so a hit can say what it is and be narrowed to one profile, but they
-- are never matched - a search for "work" should not return every work.
CREATE VIRTUAL TABLE search_index USING fts5(
    entity UNINDEXED,
    entity_id UNINDEXED,
    profile_id UNINDEXED,
    work_id UNINDEXED,
    body,
    tokenize = 'unicode61 remove_diacritics 0'
);

-- A work: its title, its craft fields and its tags as one document. The meta
-- and tags columns hold JSON, so the values are flattened out of them -
-- indexing the raw JSON would match on key names and on punctuation. Only
-- scalars are taken: a nested object has no words of its own.
--
-- The type comes from `json_each`'s own `type` column, not from
-- `json_type(value)`: by the time `value` is in hand it is an unwrapped
-- scalar, so `json_type` tries to parse `Female, remote and intimate` as a
-- document and fails the whole statement with "malformed JSON".
CREATE TRIGGER search_work_on_insert AFTER INSERT ON work
BEGIN
    INSERT INTO search_index (entity, entity_id, profile_id, work_id, body)
    VALUES ('work', NEW.id, NEW.profile_id, NEW.id,
        NEW.title || ' ' ||
        coalesce((SELECT group_concat(value, ' ') FROM json_each(NEW.meta)
                   WHERE type IN ('text', 'integer', 'real')), '') || ' ' ||
        coalesce((SELECT group_concat(value, ' ') FROM json_each(NEW.tags)), ''));
END;

CREATE TRIGGER search_work_on_update AFTER UPDATE ON work
WHEN OLD.title IS NOT NEW.title OR OLD.meta IS NOT NEW.meta OR OLD.tags IS NOT NEW.tags
BEGIN
    DELETE FROM search_index WHERE entity = 'work' AND entity_id = NEW.id;
    INSERT INTO search_index (entity, entity_id, profile_id, work_id, body)
    VALUES ('work', NEW.id, NEW.profile_id, NEW.id,
        NEW.title || ' ' ||
        coalesce((SELECT group_concat(value, ' ') FROM json_each(NEW.meta)
                   WHERE type IN ('text', 'integer', 'real')), '') || ' ' ||
        coalesce((SELECT group_concat(value, ' ') FROM json_each(NEW.tags)), ''));
END;

CREATE TRIGGER search_work_on_delete AFTER DELETE ON work
BEGIN
    DELETE FROM search_index WHERE entity = 'work' AND entity_id = OLD.id;
END;

-- A version body. Bodies are edited in place (ADR 0015), so the update trigger
-- carries as much weight as the insert one.
CREATE TRIGGER search_version_on_insert AFTER INSERT ON work_version
BEGIN
    INSERT INTO search_index (entity, entity_id, profile_id, work_id, body)
    SELECT 'version', NEW.id, w.profile_id, NEW.work_id, NEW.body
      FROM work w WHERE w.id = NEW.work_id;
END;

CREATE TRIGGER search_version_on_update AFTER UPDATE ON work_version
WHEN OLD.body IS NOT NEW.body
BEGIN
    DELETE FROM search_index WHERE entity = 'version' AND entity_id = NEW.id;
    INSERT INTO search_index (entity, entity_id, profile_id, work_id, body)
    SELECT 'version', NEW.id, w.profile_id, NEW.work_id, NEW.body
      FROM work w WHERE w.id = NEW.work_id;
END;

CREATE TRIGGER search_version_on_delete AFTER DELETE ON work_version
BEGIN
    DELETE FROM search_index WHERE entity = 'version' AND entity_id = OLD.id;
END;

-- A note. `work_id` is nullable here and stays nullable in the index: a note
-- about nothing in particular is still worth finding, and the search it
-- replaces dropped exactly those, by joining inwards to `work`.
CREATE TRIGGER search_note_on_insert AFTER INSERT ON note
BEGIN
    INSERT INTO search_index (entity, entity_id, profile_id, work_id, body)
    VALUES ('note', NEW.id, NEW.profile_id, NEW.work_id,
        coalesce(NEW.title, '') || ' ' || NEW.body || ' ' ||
        coalesce((SELECT group_concat(value, ' ') FROM json_each(NEW.tags)), ''));
END;

CREATE TRIGGER search_note_on_update AFTER UPDATE ON note
WHEN OLD.title IS NOT NEW.title OR OLD.body IS NOT NEW.body OR OLD.tags IS NOT NEW.tags
BEGIN
    DELETE FROM search_index WHERE entity = 'note' AND entity_id = NEW.id;
    INSERT INTO search_index (entity, entity_id, profile_id, work_id, body)
    VALUES ('note', NEW.id, NEW.profile_id, NEW.work_id,
        coalesce(NEW.title, '') || ' ' || NEW.body || ' ' ||
        coalesce((SELECT group_concat(value, ' ') FROM json_each(NEW.tags)), ''));
END;

CREATE TRIGGER search_note_on_delete AFTER DELETE ON note
BEGIN
    DELETE FROM search_index WHERE entity = 'note' AND entity_id = OLD.id;
END;

-- An assistant message. Written once and never edited, so there is no update
-- trigger to write.
CREATE TRIGGER search_message_on_insert AFTER INSERT ON chat_message
BEGIN
    INSERT INTO search_index (entity, entity_id, profile_id, work_id, body)
    SELECT 'message', NEW.id, c.profile_id, c.work_id, NEW.body
      FROM chat c WHERE c.id = NEW.chat_id;
END;

CREATE TRIGGER search_message_on_delete AFTER DELETE ON chat_message
BEGIN
    DELETE FROM search_index WHERE entity = 'message' AND entity_id = OLD.id;
END;

-- Fill the index from what is already here. The triggers only see what happens
-- next, and a workspace in use for months is entirely "before".
INSERT INTO search_index (entity, entity_id, profile_id, work_id, body)
SELECT 'work', work.id, work.profile_id, work.id,
    work.title || ' ' ||
    coalesce((SELECT group_concat(value, ' ') FROM json_each(work.meta)
               WHERE type IN ('text', 'integer', 'real')), '') || ' ' ||
    coalesce((SELECT group_concat(value, ' ') FROM json_each(work.tags)), '')
  FROM work;

INSERT INTO search_index (entity, entity_id, profile_id, work_id, body)
SELECT 'version', v.id, w.profile_id, v.work_id, v.body
  FROM work_version v JOIN work w ON w.id = v.work_id;

INSERT INTO search_index (entity, entity_id, profile_id, work_id, body)
SELECT 'note', note.id, note.profile_id, note.work_id,
    coalesce(note.title, '') || ' ' || note.body || ' ' ||
    coalesce((SELECT group_concat(value, ' ') FROM json_each(note.tags)), '')
  FROM note;

INSERT INTO search_index (entity, entity_id, profile_id, work_id, body)
SELECT 'message', m.id, c.profile_id, c.work_id, m.body
  FROM chat_message m JOIN chat c ON c.id = m.chat_id;

-- Every work that shipped is finished by definition, so the stage that took
-- months to earn is not left for the author to click in one by one. Only
-- released works: a booked release is a plan, not a finished piece.
UPDATE work SET stage = 100
 WHERE stage IS NULL
   AND EXISTS (SELECT 1 FROM release r WHERE r.work_id = work.id AND r.released_at IS NOT NULL);
