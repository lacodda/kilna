-- # Two shelves (ADR 0064)
--
-- The material for works to come was four places: ideas, phrases, notes and
-- a bank of words. An idea differed from a note by one flag, a word of the
-- bank from a phrase by being a row of another table. They become two:
-- a phrase - a word, a line, a quote, whatever turns up in a head as one line
-- - and a note - a thought, from a seed to a written page.
--
-- So a word of the bank becomes a phrase, the bank's blocks become shelves of
-- phrases, an idea becomes a note tagged `idea`, and a term keeps only what is
-- the record's own: how strictly it is spent and how it is sung.

-- ## Whose words a phrase is
--
-- A quote is a phrase with an author: "печаль - это изношенная радость",
-- from "Old Joy". The author is not the phrase's tag (a tag is a facet many
-- phrases share) and not its title (a phrase has none): a column of its own.
ALTER TABLE note ADD COLUMN attribution TEXT;

-- The note's clock names its columns, so the new one joins it. Dropped and
-- written whole: SQLite has no ALTER TRIGGER (the lesson of 0012).
DROP TRIGGER note_clock_on_update;

CREATE TRIGGER note_clock_on_update AFTER UPDATE ON note
BEGIN
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'note', NEW.id, 'work_id', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.work_id IS NOT NEW.work_id
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'note', NEW.id, 'kind', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.kind IS NOT NEW.kind
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'note', NEW.id, 'title', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.title IS NOT NEW.title
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'note', NEW.id, 'body', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.body IS NOT NEW.body
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'note', NEW.id, 'tags', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.tags IS NOT NEW.tags
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'note', NEW.id, 'layer', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.layer IS NOT NEW.layer
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'note', NEW.id, 'aliases', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.aliases IS NOT NEW.aliases
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'note', NEW.id, 'prompt', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.prompt IS NOT NEW.prompt
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'note', NEW.id, 'prompt_basis', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.prompt_basis IS NOT NEW.prompt_basis
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'note', NEW.id, 'state', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.state IS NOT NEW.state
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'note', NEW.id, 'attribution', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.attribution IS NOT NEW.attribution
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
END;

-- A quote is found by its author as well as by its words.
DROP TRIGGER search_note_on_insert;
DROP TRIGGER search_note_on_update;

CREATE TRIGGER search_note_on_insert AFTER INSERT ON note
BEGIN
    INSERT INTO search_index (entity, entity_id, profile_id, work_id, body)
    VALUES ('note', NEW.id, NEW.profile_id, NEW.work_id,
        coalesce(NEW.title, '') || ' ' || NEW.body || ' ' || coalesce(NEW.attribution, '') || ' ' ||
        coalesce((SELECT group_concat(value, ' ') FROM json_each(NEW.tags)), '') || ' ' ||
        coalesce((SELECT group_concat(value, ' ') FROM json_each(NEW.aliases)), ''));
END;

CREATE TRIGGER search_note_on_update AFTER UPDATE ON note
WHEN OLD.title IS NOT NEW.title OR OLD.body IS NOT NEW.body OR OLD.tags IS NOT NEW.tags
     OR OLD.aliases IS NOT NEW.aliases OR OLD.work_id IS NOT NEW.work_id
     OR OLD.attribution IS NOT NEW.attribution
BEGIN
    DELETE FROM search_index WHERE entity = 'note' AND entity_id = NEW.id;
    INSERT INTO search_index (entity, entity_id, profile_id, work_id, body)
    VALUES ('note', NEW.id, NEW.profile_id, NEW.work_id,
        coalesce(NEW.title, '') || ' ' || NEW.body || ' ' || coalesce(NEW.attribution, '') || ' ' ||
        coalesce((SELECT group_concat(value, ' ') FROM json_each(NEW.tags)), '') || ' ' ||
        coalesce((SELECT group_concat(value, ' ') FROM json_each(NEW.aliases)), ''));
END;

-- ## Shelves of phrases
--
-- The owner sorts what they keep the way they think of it - "space", "for
-- the slow one", the song it is gathered for - in an order they choose, and a
-- phrase may stand on several shelves. The bank's blocks, holding notes now
-- rather than terms: a shelf is a row with a position, a phrase on a shelf a
-- row of the pair.
CREATE TABLE shelf (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profile (id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (trim(name) <> '')
) STRICT;

CREATE INDEX shelf_profile ON shelf (profile_id, position);

CREATE TABLE shelf_note (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profile (id) ON DELETE CASCADE,
    shelf_id TEXT NOT NULL REFERENCES shelf (id) ON DELETE CASCADE,
    note_id TEXT NOT NULL REFERENCES note (id) ON DELETE CASCADE,
    created_at TEXT NOT NULL,
    UNIQUE (shelf_id, note_id)
) STRICT;

CREATE INDEX shelf_note_note ON shelf_note (note_id);

CREATE TRIGGER shelf_tombstone_on_delete AFTER DELETE ON shelf
BEGIN
    INSERT INTO tombstone (entity, entity_id, profile_id, label, deleted_at, deleted_by)
    VALUES ('shelf', OLD.id, OLD.profile_id, OLD.name, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device))
    ON CONFLICT (entity, entity_id) DO UPDATE SET
        profile_id = excluded.profile_id, label = excluded.label,
        deleted_at = excluded.deleted_at, deleted_by = excluded.deleted_by,
        restored_at = NULL, restored_by = NULL;
    DELETE FROM field_clock WHERE entity = 'shelf' AND entity_id = OLD.id;
END;

CREATE TRIGGER shelf_tombstone_on_insert AFTER INSERT ON shelf
BEGIN
    UPDATE tombstone SET restored_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), restored_by = (SELECT id FROM device)
    WHERE entity = 'shelf' AND entity_id = NEW.id AND restored_at IS NULL;
END;

CREATE TRIGGER shelf_clock_on_update AFTER UPDATE ON shelf
BEGIN
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'shelf', NEW.id, 'name', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.name IS NOT NEW.name
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'shelf', NEW.id, 'position', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.position IS NOT NEW.position
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
END;

CREATE TRIGGER shelf_note_tombstone_on_delete AFTER DELETE ON shelf_note
BEGIN
    INSERT INTO tombstone (entity, entity_id, profile_id, label, deleted_at, deleted_by)
    VALUES ('shelf_note', OLD.id, OLD.profile_id, (SELECT substr(body, 1, 80) FROM note WHERE id = OLD.note_id), strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device))
    ON CONFLICT (entity, entity_id) DO UPDATE SET
        profile_id = excluded.profile_id, label = excluded.label,
        deleted_at = excluded.deleted_at, deleted_by = excluded.deleted_by,
        restored_at = NULL, restored_by = NULL;
END;

CREATE TRIGGER shelf_note_tombstone_on_insert AFTER INSERT ON shelf_note
BEGIN
    UPDATE tombstone SET restored_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), restored_by = (SELECT id FROM device)
    WHERE entity = 'shelf_note' AND entity_id = NEW.id AND restored_at IS NULL;
END;

-- The blocks come across under the ids they had, so their trail - a
-- tombstone, a field's clock - still names them.
INSERT INTO shelf (id, profile_id, name, position, created_at, updated_at)
SELECT id, profile_id, name, position, created_at, updated_at FROM term_block;

UPDATE tombstone SET entity = 'shelf' WHERE entity = 'term_block';
DELETE FROM tombstone WHERE entity = 'term_block_word';
UPDATE field_clock SET entity = 'shelf' WHERE entity = 'term_block';

-- ## A word of the bank becomes a phrase
--
-- Every word the bank kept, and every word a block still held after it left
-- the bank - that one given up on. A phrase already written in the same
-- words is the one it becomes, rather than a second copy beside it. The new
-- ids are version-4 UUIDs, the shape every other id has.
CREATE TEMP TABLE banked AS
SELECT t.id AS term_id,
       t.profile_id,
       trim(t.word) AS word,
       coalesce(t.bank, 'dropped') AS state,
       t.created_at,
       t.updated_at,
       (SELECT n.id FROM note n
         WHERE n.profile_id = t.profile_id AND n.kind = 'phrase' AND trim(n.body) = trim(t.word)
         ORDER BY n.created_at, n.rowid LIMIT 1) AS existing,
       lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-4' ||
           substr(lower(hex(randomblob(2))), 2) || '-' ||
           substr('89ab', 1 + (abs(random()) % 4), 1) || substr(lower(hex(randomblob(2))), 2) || '-' ||
           lower(hex(randomblob(6))) AS fresh_id
  FROM term t
 WHERE t.bank IS NOT NULL
    OR EXISTS (SELECT 1 FROM term_block_word w WHERE w.term_id = t.id);

INSERT INTO note (id, profile_id, work_id, kind, title, body, tags, created_at, updated_at, state)
SELECT fresh_id, profile_id, NULL, 'phrase', NULL, word, '[]', created_at, updated_at, state
  FROM banked
 WHERE existing IS NULL;

INSERT INTO shelf_note (id, profile_id, shelf_id, note_id, created_at)
SELECT w.id, w.profile_id, w.block_id, coalesce(b.existing, b.fresh_id), w.created_at
  FROM term_block_word w
  JOIN banked b ON b.term_id = w.term_id
 WHERE true
    ON CONFLICT (shelf_id, note_id) DO NOTHING;

-- A word that was only kept, with nothing else the record says about it,
-- has said everything as a phrase now. A word that is also spent, sung its
-- own way, counted in other forms or named for a work stays a term.
DELETE FROM term
 WHERE id IN (SELECT term_id FROM banked)
   AND strictness IS NULL
   AND sung = '[]'
   AND forms = '[]'
   AND topic IS NULL
   AND note IS NULL
   AND NOT EXISTS (SELECT 1 FROM term_work WHERE term_work.term_id = term.id);

DROP TABLE banked;

-- The blocks go before the term is rebuilt: the trigger on their words reads
-- the term's word, and a rename refuses a schema whose triggers name a table
-- that is, for a moment, not there.
DROP TABLE term_block_word;
DROP TABLE term_block;

-- ## A term without a bank
--
-- What is left to a term is the record's own: how strictly it is spent, how
-- it is sung. The column goes, which SQLite does by writing the table anew.
CREATE TABLE term_rebuilt (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profile (id) ON DELETE CASCADE,
    word TEXT NOT NULL,
    forms TEXT NOT NULL DEFAULT '[]',
    kind TEXT NOT NULL DEFAULT 'noun',
    strictness TEXT,
    sung TEXT NOT NULL DEFAULT '[]',
    topic TEXT,
    note TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (trim(word) <> ''),
    CHECK (json_valid(forms) AND json_type(forms) = 'array'),
    CHECK (json_valid(sung) AND json_type(sung) = 'array'),
    CHECK (kind IN ('noun', 'adjective', 'verb', 'phrase', 'image', 'scene', 'pattern')),
    CHECK (strictness IS NULL OR strictness IN ('ban', 'limit', 'rare'))
) STRICT;

INSERT INTO term_rebuilt (id, profile_id, word, forms, kind, strictness, sung, topic, note,
                          created_at, updated_at)
SELECT id, profile_id, word, forms, kind, strictness, sung, topic, note, created_at, updated_at
  FROM term;

DROP TRIGGER term_work_tombstone_on_delete;
DROP TABLE term;
ALTER TABLE term_rebuilt RENAME TO term;

CREATE INDEX term_profile ON term (profile_id);

CREATE TRIGGER term_work_tombstone_on_delete AFTER DELETE ON term_work
BEGIN
    INSERT INTO tombstone (entity, entity_id, profile_id, label, deleted_at, deleted_by)
    VALUES ('term_work', OLD.id, OLD.profile_id, (SELECT word FROM term WHERE id = OLD.term_id), strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device))
    ON CONFLICT (entity, entity_id) DO UPDATE SET
        profile_id = excluded.profile_id, label = excluded.label,
        deleted_at = excluded.deleted_at, deleted_by = excluded.deleted_by,
        restored_at = NULL, restored_by = NULL;
END;

CREATE TRIGGER term_tombstone_on_delete AFTER DELETE ON term
BEGIN
    INSERT INTO tombstone (entity, entity_id, profile_id, label, deleted_at, deleted_by)
    VALUES ('term', OLD.id, OLD.profile_id, OLD.word, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device))
    ON CONFLICT (entity, entity_id) DO UPDATE SET
        profile_id = excluded.profile_id, label = excluded.label,
        deleted_at = excluded.deleted_at, deleted_by = excluded.deleted_by,
        restored_at = NULL, restored_by = NULL;
    DELETE FROM field_clock WHERE entity = 'term' AND entity_id = OLD.id;
END;

CREATE TRIGGER term_tombstone_on_insert AFTER INSERT ON term
BEGIN
    UPDATE tombstone SET restored_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), restored_by = (SELECT id FROM device)
    WHERE entity = 'term' AND entity_id = NEW.id AND restored_at IS NULL;
END;

CREATE TRIGGER term_clock_on_update AFTER UPDATE ON term
BEGIN
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'term', NEW.id, 'word', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.word IS NOT NEW.word
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'term', NEW.id, 'forms', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.forms IS NOT NEW.forms
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'term', NEW.id, 'kind', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.kind IS NOT NEW.kind
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'term', NEW.id, 'strictness', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.strictness IS NOT NEW.strictness
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'term', NEW.id, 'sung', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.sung IS NOT NEW.sung
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'term', NEW.id, 'topic', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.topic IS NOT NEW.topic
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'term', NEW.id, 'note', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.note IS NOT NEW.note
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
END;

DELETE FROM field_clock WHERE entity = 'term' AND field = 'bank';

-- What the trash holds is restored by the columns its copy names, so a copy
-- taken before this step says what the tables say now: a term without its
-- bank and the blocks it stood in, a block as a shelf without the words -
-- those are phrases now, and stand on the shelf by themselves.
UPDATE deletion
   SET snapshot = json_remove(
           json_set(snapshot, '$.term',
               json((SELECT json_group_array(json(json_remove(value, '$.bank')))
                       FROM json_each(deletion.snapshot, '$.term')))),
           '$.term_block_word')
 WHERE entity = 'term' AND json_type(snapshot, '$.term') = 'array';

UPDATE deletion
   SET entity = 'shelf',
       snapshot = json_remove(
           json_set(snapshot, '$.shelf', json(json_extract(snapshot, '$.term_block'))),
           '$.term_block', '$.term_block_word')
 WHERE entity = 'block';

-- ## An idea becomes a note
--
-- A note that was an idea keeps saying so by a tag, so the seeds of works can
-- still be found in one click. A note is not spent by works the way a phrase
-- is: one that went into a work is tied to it, and stands fresh.
UPDATE note
   SET kind = 'note',
       state = 'fresh',
       tags = CASE
                  WHEN EXISTS (SELECT 1 FROM json_each(note.tags) WHERE value = 'idea') THEN tags
                  ELSE json_insert(tags, '$[#]', 'idea')
              END
 WHERE kind = 'idea';

-- And so does an idea in the trash, deleted alone or with its work: it comes
-- back as the note it would be now. A note's tags are a column of text
-- holding JSON, so they are written back as text, not as an array.
UPDATE deletion
   SET snapshot = json_set(snapshot, '$.note', json((
           SELECT json_group_array(json(
                      CASE
                          WHEN json_extract(r.value, '$.kind') = 'idea' THEN
                              json_set(r.value,
                                  '$.kind', 'note',
                                  '$.state', 'fresh',
                                  '$.tags', CASE
                                      WHEN EXISTS (SELECT 1 FROM json_each(json_extract(r.value, '$.tags')) t
                                                    WHERE t.value = 'idea')
                                          THEN json_extract(r.value, '$.tags')
                                      ELSE '' || json_insert(json_extract(r.value, '$.tags'), '$[#]', 'idea')
                                  END)
                          ELSE r.value
                      END))
             FROM json_each(deletion.snapshot, '$.note') r)))
 WHERE json_type(snapshot, '$.note') = 'array'
   AND EXISTS (SELECT 1 FROM json_each(deletion.snapshot, '$.note') r
                WHERE json_extract(r.value, '$.kind') = 'idea');
