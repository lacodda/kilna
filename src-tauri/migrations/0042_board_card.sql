-- The boards of a work's conception (v0.99, ADR 0065).
--
-- Before a work is written it is prepared: the person says what it is about
-- and how they see it (the brief), and boards gather candidates for it -
-- scenarios to write it by, sources to draw on - each starred, turned down
-- or left. The profile says which boards a kind of work lays out, the kinds
-- of card each holds and the fields of each kind; a card is a row of this
-- table, whatever the board.
--
-- ## One table for every board
--
-- A cover's ideas (0034) and an experiment's trials (0038) are boards with
-- a table each, because what is on them has a shape of its own - a cover's
-- concept, a text that is run and heard. A card of a conception is words
-- in named fields, and a board of scenarios differs from a board of sources
-- only in the names: one table, the board and the kind of card as keys the
-- profile owns, the fields as one JSON object by the field keys of the kind.
--
-- ## Where it stands
--
-- `position` orders the whole board, and `topic` is the group it stands in -
-- free words, the board's own, like a trial's series: what a card is grouped
-- under is a word on the card rather than a row of its own, so renaming a
-- topic, or merging two, is giving their cards one word. A board shows its
-- topics in the order their first cards stand. Empty is no topic.
--
-- ## Where it came from, and the verdict
--
-- `source` is one of three words: `own` is the person's card in their words,
-- `refined` is one of theirs worked out by the assistant, `ai` is the
-- assistant's own. One verdict, or none: starred, turned down, or not judged
-- yet - one column with two words, as a cover's ideas have (0034).
--
-- Not in the search, as a cover's ideas are not: a card is a candidate.

CREATE TABLE board_card (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profile (id) ON DELETE CASCADE,
    -- The work whose board it is on. Goes with the work to the trash and
    -- comes back with it (see `trash::cascade`).
    work_id TEXT NOT NULL REFERENCES work (id) ON DELETE CASCADE,
    -- The board, by its key in the profile: `scenarios`.
    board TEXT NOT NULL,
    -- The kind of card, by its key on the board: `scenario`.
    kind TEXT NOT NULL,
    topic TEXT NOT NULL DEFAULT '',
    position INTEGER NOT NULL DEFAULT 0,
    source TEXT NOT NULL,
    verdict TEXT,
    -- What the card says, by the field keys of its kind.
    fields TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (source IN ('own', 'refined', 'ai')),
    CHECK (verdict IS NULL OR verdict IN ('star', 'rejected')),
    CHECK (json_valid(fields) AND json_type(fields) = 'object')
);

-- A work's board, in order.
CREATE INDEX board_card_work ON board_card (work_id, board, position);

-- Tombstones, the shape 0011 gives every table a person's work lives in. The
-- label is the card's headline - the first of its fields that says anything,
-- in the order the object holds them.
CREATE TRIGGER board_card_tombstone_on_delete AFTER DELETE ON board_card
BEGIN
    INSERT INTO tombstone (entity, entity_id, profile_id, label, deleted_at, deleted_by)
    VALUES ('board_card', OLD.id, OLD.profile_id,
        substr(coalesce((SELECT value FROM json_each(OLD.fields)
            WHERE type = 'text' AND trim(value) <> '' LIMIT 1), ''), 1, 80),
        strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device))
    ON CONFLICT (entity, entity_id) DO UPDATE SET
        profile_id = excluded.profile_id, label = excluded.label,
        deleted_at = excluded.deleted_at, deleted_by = excluded.deleted_by,
        restored_at = NULL, restored_by = NULL;
    DELETE FROM field_clock WHERE entity = 'board_card' AND entity_id = OLD.id;
END;

CREATE TRIGGER board_card_tombstone_on_insert AFTER INSERT ON board_card
BEGIN
    UPDATE tombstone SET restored_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), restored_by = (SELECT id FROM device)
    WHERE entity = 'board_card' AND entity_id = NEW.id AND restored_at IS NULL;
END;

-- Field clocks, so a card starred on one machine and moved on another
-- settles the way every other edit settles (0011). One statement per column:
-- a column left out here is invisible to synchronisation (the lesson of 0023).
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
END;
