-- # The library (v0.100, ADR 0066)
--
-- A work is researched before it is written: the works of culture it could
-- draw on - a novel, a film, a painting, a gospel - each with why it belongs
-- and what to take from it. The same film turns up in the research of two
-- songs, and it is one film: a work of culture is one entry of the library
-- for the whole workspace, a note of the kind the profile marks `library`,
-- and a card on a work's board links to it with what it is to that work.

-- ## What an entry of the library is
--
-- Its title is the note's title, its author the note's attribution ("whose
-- words": the novelist, the director, the painter), its other titles - the
-- original, a translation - the note's aliases, what it is about its body.
-- Two things more are its own: the medium it is in, by the key of one the
-- library's kind declares (literature, film, music), and when it was made,
-- as said: "1913", "c. 1308", "1880-83". Columns rather than words in the
-- title, so an entry is found again by its name whatever its year says.
ALTER TABLE note ADD COLUMN medium TEXT;
ALTER TABLE note ADD COLUMN year TEXT;

-- The note's clock names its columns, so the new ones join it. Dropped and
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
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'note', NEW.id, 'medium', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.medium IS NOT NEW.medium
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'note', NEW.id, 'year', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.year IS NOT NEW.year
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
END;

-- ## A card that links to an entry
--
-- A card of a kind the board marks `library` says what the entry is to this
-- work - why it is here, what to take, the quote and where it was checked -
-- and nothing of what the entry is: its title and author are read from the
-- entry, never copied, so an entry put right is put right on every board.
--
-- Set to nothing when the entry goes, as a version's parent is: a card
-- restored from the trash after its entry was deleted comes back naming
-- nothing rather than failing on the key. An entry a board still links to is
-- not deleted at all - the action refuses it, naming the works.
ALTER TABLE board_card ADD COLUMN note_id TEXT REFERENCES note (id) ON DELETE SET NULL;

-- Where an entry has worked: every card that links to it, across the works.
CREATE INDEX board_card_note ON board_card (note_id) WHERE note_id IS NOT NULL;

-- The card's tombstone is labelled by its headline, and a card that links to
-- an entry is called what the entry is called.
DROP TRIGGER board_card_tombstone_on_delete;

CREATE TRIGGER board_card_tombstone_on_delete AFTER DELETE ON board_card
BEGIN
    INSERT INTO tombstone (entity, entity_id, profile_id, label, deleted_at, deleted_by)
    VALUES ('board_card', OLD.id, OLD.profile_id,
        substr(coalesce(
            (SELECT nullif(trim(title), '') FROM note WHERE id = OLD.note_id),
            (SELECT value FROM json_each(OLD.fields) WHERE type = 'text' AND trim(value) <> '' LIMIT 1),
            ''), 1, 80),
        strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device))
    ON CONFLICT (entity, entity_id) DO UPDATE SET
        profile_id = excluded.profile_id, label = excluded.label,
        deleted_at = excluded.deleted_at, deleted_by = excluded.deleted_by,
        restored_at = NULL, restored_by = NULL;
    DELETE FROM field_clock WHERE entity = 'board_card' AND entity_id = OLD.id;
END;

-- The card's clock names its columns, so the link joins it.
DROP TRIGGER board_card_clock_on_update;

CREATE TRIGGER board_card_clock_on_update AFTER UPDATE ON board_card
BEGIN
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'board_card', NEW.id, 'board', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.board IS NOT NEW.board
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'board_card', NEW.id, 'kind', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.kind IS NOT NEW.kind
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'board_card', NEW.id, 'topic', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.topic IS NOT NEW.topic
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'board_card', NEW.id, 'position', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.position IS NOT NEW.position
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'board_card', NEW.id, 'source', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.source IS NOT NEW.source
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'board_card', NEW.id, 'verdict', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.verdict IS NOT NEW.verdict
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'board_card', NEW.id, 'fields', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.fields IS NOT NEW.fields
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'board_card', NEW.id, 'note_id', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.note_id IS NOT NEW.note_id
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
END;
