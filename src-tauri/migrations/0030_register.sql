-- The register of repeats, and the state of a note that is material.
--
-- ## The register
--
-- A body of work repeats itself: the same words, images and scenes come back
-- until a listener hears one voice saying one thing. The register records what
-- is spent - a term, how strictly it is off limits, and what it is - so a text
-- can be checked against it while it is written (ADR 0044).
--
-- A term is either wording or a meaning. Wording (a noun, an adjective, a
-- verb, a phrase) is found in a text by its forms, so which works carry it is
-- read off the works every time it is asked and is never written down: a count
-- typed by hand is out of date the day after a new song. A meaning (an image,
-- a scene, a pattern) cannot be found by its words - "a lighthouse nobody keeps" is
-- said a hundred ways - so the works that carry it are rows of
-- `term_work`, drawn by a person or kept from a proposal. A row may name a
-- work for wording too, where the text says it in a form the term does not
-- list; the works a term is in are the ones found and the ones named, each
-- once.
--
-- `forms` - the other words counted as the same term ("окна" beside "окно",
-- "горячий" beside "тёплый"), as a JSON array. Cases need no form: the
-- words are compared by their stems.
--
-- `kind` and `strictness` are the register's own words, not the profile's:
-- the code reads both. A kind decides whether a term is looked for in a text;
-- a strictness decides how a hit is marked - a ban stands out, a limit is
-- underlined, a rare word is dotted.
--
-- `topic` - the register's own grouping ("the kitchen", "physics and space"),
-- free text, one per term.
CREATE TABLE term (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profile (id) ON DELETE CASCADE,
    word TEXT NOT NULL,
    forms TEXT NOT NULL DEFAULT '[]',
    kind TEXT NOT NULL DEFAULT 'noun',
    strictness TEXT NOT NULL DEFAULT 'limit',
    topic TEXT,
    note TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (trim(word) <> ''),
    CHECK (json_valid(forms) AND json_type(forms) = 'array'),
    CHECK (kind IN ('noun', 'adjective', 'verb', 'phrase', 'image', 'scene', 'pattern')),
    CHECK (strictness IN ('ban', 'limit', 'rare'))
) STRICT;

CREATE INDEX term_profile ON term (profile_id);

-- A work that carries a term, named rather than found: the way a meaning is
-- tied to the works it is in. One row per pair.
CREATE TABLE term_work (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profile (id) ON DELETE CASCADE,
    term_id TEXT NOT NULL REFERENCES term (id) ON DELETE CASCADE,
    work_id TEXT NOT NULL REFERENCES work (id) ON DELETE CASCADE,
    created_at TEXT NOT NULL,
    UNIQUE (term_id, work_id)
) STRICT;

CREATE INDEX term_work_work ON term_work (work_id);

-- Tombstones, the shape 0011 gives every table a person's work lives in.
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

CREATE TRIGGER term_work_tombstone_on_delete AFTER DELETE ON term_work
BEGIN
    INSERT INTO tombstone (entity, entity_id, profile_id, label, deleted_at, deleted_by)
    VALUES ('term_work', OLD.id, OLD.profile_id, (SELECT word FROM term WHERE id = OLD.term_id), strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device))
    ON CONFLICT (entity, entity_id) DO UPDATE SET
        profile_id = excluded.profile_id, label = excluded.label,
        deleted_at = excluded.deleted_at, deleted_by = excluded.deleted_by,
        restored_at = NULL, restored_by = NULL;
END;

CREATE TRIGGER term_work_tombstone_on_insert AFTER INSERT ON term_work
BEGIN
    UPDATE tombstone SET restored_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), restored_by = (SELECT id FROM device)
    WHERE entity = 'term_work' AND entity_id = NEW.id AND restored_at IS NULL;
END;

-- Field clocks: one statement per column, or the column is invisible to
-- synchronisation (the lesson of 0023).
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
    SELECT 'term', NEW.id, 'topic', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.topic IS NOT NEW.topic
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'term', NEW.id, 'note', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.note IS NOT NEW.note
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
END;

-- ## A note that is material
--
-- An idea and a phrase are notes (ADR 0025) of kinds the profile marks as
-- material: what works are made from, and spent by them. A phrase used in a
-- song stays in the bank, marked used and tied to the song, so it is not used
-- a second time - which is the whole reason for a bank of phrases (ADR 0045).
--
-- `state` is `fresh` until the note is spent, set aside or given up on. Every
-- note carries it; only the material kinds show it, and a plain note stays
-- fresh for ever, which is harmless. The words are the code's, not the
-- profile's: "to a work" sets `used`.
ALTER TABLE note ADD COLUMN state TEXT NOT NULL DEFAULT 'fresh'
    CHECK (state IN ('fresh', 'used', 'parked', 'dropped'));

CREATE INDEX note_kind_state ON note (profile_id, kind, state);

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
END;
