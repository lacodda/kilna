-- The canon: the world a channel's works are made from, kept as facts.
--
-- A card of the canon is a note (ADR 0025, 0043). A character, a place, an
-- object, the channel itself - each is a note of a kind whose profile entry
-- names sections, so the scenes that already point at characters through
-- `scene_note` point at cards the day this runs, and nothing moves. What a
-- card knows is not its body: the body stays as the card's free note, and
-- the knowledge is rows of `canon_fact` - short statements, each with the
-- layer it may be told in, how settled it is, where it came from and when it
-- happened in the world. A paragraph cannot be filtered by who may read it;
-- a row can.
--
-- ## A card's own columns
--
-- `layer` - who may know the card exists at all. A person who publicly does
-- not exist is a card of the internal layer, and a task that may only read
-- the public layer does not see a single fact of it, whatever their layers.
--
-- `aliases` - the names the card goes by, as a JSON array: the forms a text
-- uses. "Where it appears" finds a card in the works' texts by these words,
-- whole words only, and there is no Russian stemmer to find the rest.
--
-- `prompt` and `prompt_basis` - the English description a picture generator
-- is given instead of the name, and a fingerprint of the facts it was written
-- from. When the facts move on, the fingerprint no longer matches and the
-- description says it is stale; nothing rewrites it behind the person's back.
ALTER TABLE note ADD COLUMN layer TEXT NOT NULL DEFAULT 'public'
    CHECK (layer IN ('public', 'internal', 'inWorks'));
ALTER TABLE note ADD COLUMN aliases TEXT NOT NULL DEFAULT '[]';
ALTER TABLE note ADD COLUMN prompt TEXT;
ALTER TABLE note ADD COLUMN prompt_basis TEXT;

-- A fact of a card.
--
-- `section` is a key of the card kind's `sections` in the profile, not a
-- foreign key: the vocabulary is a document, and a fact written under a
-- section the profile later drops still says what it said (the rule scores
-- follow for their axes, migration 0012).
--
-- The layers are the canon's own three, not the profile's words, because the
-- code reads them: a cover sees `public`, a text of a work sees all three -
-- `internal` without addresses, `inWorks` only inside the work itself.
--
-- A retired fact keeps its words and says why it was retired: "the reference
-- had a ponytail; the loose hair stays" is the whole reason the next picture
-- does not bring the ponytail back. A retirement without a reason cannot be
-- stored, and a live fact cannot carry one.
--
-- The source is a soft reference. A fact taken from a song keeps the song's
-- id and the line it was read off, but the song may go to the trash and come
-- back, and a foreign key would have forgotten it on the way out. The label is
-- what is shown when the work is not there - and all there is for a decision
-- or a document.
--
-- Time in the world is two things: the words it is told in ("winter
-- 2022/23") and a partial date the timeline sorts by ("2022-12"). Either may
-- be absent; only a fact with the second has a place on the line.
--
-- `scope_work_id` narrows a fact to one work: "for this release: a burgundy
-- hoodie". Soft, for the source's reason.
--
-- `data` holds what a section of another shape needs beside the words: the
-- colour of a palette entry, the slot of a caption, the template and switch
-- of a signature detail, the code of a mark, the brick of a house style. The
-- profile names the section's shape and the application checks the object
-- against it on write - the way it checks a work's fields.
CREATE TABLE canon_fact (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profile (id) ON DELETE CASCADE,
    note_id TEXT NOT NULL REFERENCES note (id) ON DELETE CASCADE,
    section TEXT NOT NULL,
    body TEXT NOT NULL,
    layer TEXT NOT NULL DEFAULT 'public',
    status TEXT NOT NULL DEFAULT 'canon',
    retired_reason TEXT,
    source_kind TEXT,
    source_work_id TEXT,
    source_version_id TEXT,
    source_line TEXT,
    source_label TEXT,
    when_label TEXT,
    when_sort TEXT,
    scope_work_id TEXT,
    data TEXT NOT NULL DEFAULT '{}',
    position INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (trim(section) <> ''),
    CHECK (trim(body) <> ''),
    CHECK (layer IN ('public', 'internal', 'inWorks')),
    CHECK (status IN ('canon', 'open', 'draft', 'retired')),
    CHECK (status <> 'retired' OR trim(coalesce(retired_reason, '')) <> ''),
    CHECK (status = 'retired' OR retired_reason IS NULL),
    CHECK (source_kind IS NULL OR source_kind IN ('work', 'decision', 'document')),
    CHECK (source_kind IS NOT 'work' OR source_work_id IS NOT NULL),
    CHECK (source_kind NOT IN ('decision', 'document') OR trim(coalesce(source_label, '')) <> ''),
    CHECK (source_kind IS NOT NULL OR (source_work_id IS NULL AND source_version_id IS NULL
        AND source_line IS NULL AND source_label IS NULL)),
    CHECK (when_sort IS NULL OR when_sort GLOB '[0-9][0-9][0-9][0-9]'
        OR when_sort GLOB '[0-9][0-9][0-9][0-9]-[01][0-9]'
        OR when_sort GLOB '[0-9][0-9][0-9][0-9]-[01][0-9]-[0-3][0-9]'),
    CHECK (json_valid(data) AND json_type(data) = 'object')
) STRICT;

-- A card read section by section, in the order the person arranged.
CREATE INDEX canon_fact_card ON canon_fact (note_id, section, position);
-- The timeline of a workspace.
CREATE INDEX canon_fact_when ON canon_fact (profile_id, when_sort);
-- "Where it appears": the works the facts were read from.
CREATE INDEX canon_fact_source ON canon_fact (source_work_id);

-- A relation between two cards: a graph, not a paragraph.
--
-- One row per pair of cards, whichever way it was drawn: "Otto is Wren's
-- neighbour" and "Wren is Otto's neighbour" are one relation, and two rows
-- for it would be two places for it to disagree with itself. The pair is held
-- unique in both orders by the index below. The words differ by side - Wren
-- is Otto's neighbour, Otto is Wren's neighbour *and first listener* - so the
-- row carries both: `label` is what `to` is to `from`, `back_label` what
-- `from` is to `to`.
--
-- `kind` is a key of the profile's `relation_kinds` (family, neighbour, pet,
-- partner), a string for the same reason a fact's section is.
CREATE TABLE canon_link (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profile (id) ON DELETE CASCADE,
    from_id TEXT NOT NULL REFERENCES note (id) ON DELETE CASCADE,
    to_id TEXT NOT NULL REFERENCES note (id) ON DELETE CASCADE,
    kind TEXT,
    label TEXT,
    back_label TEXT,
    layer TEXT NOT NULL DEFAULT 'public',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (from_id <> to_id),
    CHECK (layer IN ('public', 'internal', 'inWorks'))
) STRICT;

CREATE UNIQUE INDEX canon_link_pair ON canon_link (min(from_id, to_id), max(from_id, to_id));
CREATE INDEX canon_link_to ON canon_link (to_id);

-- A picture of a card, or of one fact of it - an outfit, a variant of a mark.
-- Its role (portrait, reference, outfit, mood, still, mark) is the asset's
-- `kind`, the column that already says what a file is for.
ALTER TABLE asset ADD COLUMN note_id TEXT REFERENCES note (id) ON DELETE CASCADE;
ALTER TABLE asset ADD COLUMN canon_fact_id TEXT REFERENCES canon_fact (id) ON DELETE CASCADE;

CREATE INDEX asset_note ON asset (note_id);
CREATE INDEX asset_canon_fact ON asset (canon_fact_id);

-- A picture's role can be changed after it arrived, so it is clocked: a role
-- changed on one machine and a caption on another settle field by field
-- (0011). An asset had no update until now, and so no clock.
CREATE TRIGGER asset_clock_on_update AFTER UPDATE ON asset
BEGIN
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'asset', NEW.id, 'kind', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.kind IS NOT NEW.kind
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'asset', NEW.id, 'label', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.label IS NOT NEW.label
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
END;

-- The note's clock names its columns, so the four new ones join it. Dropped
-- and written whole: SQLite has no ALTER TRIGGER (the lesson of 0012).
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
END;

-- A card is found by the names it goes by, as well as by its title and note.
DROP TRIGGER search_note_on_insert;
DROP TRIGGER search_note_on_update;

CREATE TRIGGER search_note_on_insert AFTER INSERT ON note
BEGIN
    INSERT INTO search_index (entity, entity_id, profile_id, work_id, body)
    VALUES ('note', NEW.id, NEW.profile_id, NEW.work_id,
        coalesce(NEW.title, '') || ' ' || NEW.body || ' ' ||
        coalesce((SELECT group_concat(value, ' ') FROM json_each(NEW.tags)), '') || ' ' ||
        coalesce((SELECT group_concat(value, ' ') FROM json_each(NEW.aliases)), ''));
END;

CREATE TRIGGER search_note_on_update AFTER UPDATE ON note
WHEN OLD.title IS NOT NEW.title OR OLD.body IS NOT NEW.body OR OLD.tags IS NOT NEW.tags
     OR OLD.aliases IS NOT NEW.aliases OR OLD.work_id IS NOT NEW.work_id
BEGIN
    DELETE FROM search_index WHERE entity = 'note' AND entity_id = NEW.id;
    INSERT INTO search_index (entity, entity_id, profile_id, work_id, body)
    VALUES ('note', NEW.id, NEW.profile_id, NEW.work_id,
        coalesce(NEW.title, '') || ' ' || NEW.body || ' ' ||
        coalesce((SELECT group_concat(value, ' ') FROM json_each(NEW.tags)), '') || ' ' ||
        coalesce((SELECT group_concat(value, ' ') FROM json_each(NEW.aliases)), ''));
END;

-- Tombstones, the shape 0011 gives every table a person's work lives in: a
-- fact deleted here must not read as a fact that never arrived. The label is
-- its opening words, which is what a person would recognise.
CREATE TRIGGER canon_fact_tombstone_on_delete AFTER DELETE ON canon_fact
BEGIN
    INSERT INTO tombstone (entity, entity_id, profile_id, label, deleted_at, deleted_by)
    VALUES ('canon_fact', OLD.id, OLD.profile_id, substr(OLD.body, 1, 80), strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device))
    ON CONFLICT (entity, entity_id) DO UPDATE SET
        profile_id = excluded.profile_id, label = excluded.label,
        deleted_at = excluded.deleted_at, deleted_by = excluded.deleted_by,
        restored_at = NULL, restored_by = NULL;
    DELETE FROM field_clock WHERE entity = 'canon_fact' AND entity_id = OLD.id;
END;

CREATE TRIGGER canon_fact_tombstone_on_insert AFTER INSERT ON canon_fact
BEGIN
    UPDATE tombstone SET restored_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), restored_by = (SELECT id FROM device)
    WHERE entity = 'canon_fact' AND entity_id = NEW.id AND restored_at IS NULL;
END;

CREATE TRIGGER canon_link_tombstone_on_delete AFTER DELETE ON canon_link
BEGIN
    INSERT INTO tombstone (entity, entity_id, profile_id, label, deleted_at, deleted_by)
    VALUES ('canon_link', OLD.id, OLD.profile_id, OLD.label, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device))
    ON CONFLICT (entity, entity_id) DO UPDATE SET
        profile_id = excluded.profile_id, label = excluded.label,
        deleted_at = excluded.deleted_at, deleted_by = excluded.deleted_by,
        restored_at = NULL, restored_by = NULL;
    DELETE FROM field_clock WHERE entity = 'canon_link' AND entity_id = OLD.id;
END;

CREATE TRIGGER canon_link_tombstone_on_insert AFTER INSERT ON canon_link
BEGIN
    UPDATE tombstone SET restored_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), restored_by = (SELECT id FROM device)
    WHERE entity = 'canon_link' AND entity_id = NEW.id AND restored_at IS NULL;
END;

-- Field clocks: one statement per column, or the column is invisible to
-- synchronisation (the lesson of 0023). `position` too - an order changed on
-- one machine is a change like any other.
CREATE TRIGGER canon_fact_clock_on_update AFTER UPDATE ON canon_fact
BEGIN
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'canon_fact', NEW.id, 'note_id', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.note_id IS NOT NEW.note_id
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'canon_fact', NEW.id, 'section', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.section IS NOT NEW.section
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'canon_fact', NEW.id, 'body', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.body IS NOT NEW.body
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'canon_fact', NEW.id, 'layer', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.layer IS NOT NEW.layer
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'canon_fact', NEW.id, 'status', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.status IS NOT NEW.status
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'canon_fact', NEW.id, 'retired_reason', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.retired_reason IS NOT NEW.retired_reason
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'canon_fact', NEW.id, 'source_kind', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.source_kind IS NOT NEW.source_kind
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'canon_fact', NEW.id, 'source_work_id', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.source_work_id IS NOT NEW.source_work_id
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'canon_fact', NEW.id, 'source_version_id', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.source_version_id IS NOT NEW.source_version_id
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'canon_fact', NEW.id, 'source_line', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.source_line IS NOT NEW.source_line
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'canon_fact', NEW.id, 'source_label', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.source_label IS NOT NEW.source_label
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'canon_fact', NEW.id, 'when_label', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.when_label IS NOT NEW.when_label
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'canon_fact', NEW.id, 'when_sort', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.when_sort IS NOT NEW.when_sort
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'canon_fact', NEW.id, 'scope_work_id', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.scope_work_id IS NOT NEW.scope_work_id
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'canon_fact', NEW.id, 'data', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.data IS NOT NEW.data
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'canon_fact', NEW.id, 'position', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.position IS NOT NEW.position
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
END;

CREATE TRIGGER canon_link_clock_on_update AFTER UPDATE ON canon_link
BEGIN
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'canon_link', NEW.id, 'kind', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.kind IS NOT NEW.kind
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'canon_link', NEW.id, 'label', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.label IS NOT NEW.label
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'canon_link', NEW.id, 'back_label', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.back_label IS NOT NEW.back_label
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'canon_link', NEW.id, 'layer', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.layer IS NOT NEW.layer
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
END;

-- What the search finds a fact by: its words, when it happened and where it
-- came from. The hit opens the card the fact belongs to. No work travels in
-- the index: a fact is about a card, and a work whose hero card says "red
-- scarf" does not itself mention a scarf - the catalogue's "which works say
-- this" must not answer with it.
CREATE TRIGGER search_fact_on_insert AFTER INSERT ON canon_fact
BEGIN
    INSERT INTO search_index (entity, entity_id, profile_id, work_id, body)
    VALUES ('fact', NEW.id, NEW.profile_id, NULL,
        NEW.body || ' ' || coalesce(NEW.when_label, '') || ' ' || coalesce(NEW.source_label, ''));
END;

CREATE TRIGGER search_fact_on_update AFTER UPDATE ON canon_fact
WHEN OLD.body IS NOT NEW.body OR OLD.when_label IS NOT NEW.when_label
     OR OLD.source_label IS NOT NEW.source_label
BEGIN
    DELETE FROM search_index WHERE entity = 'fact' AND entity_id = NEW.id;
    INSERT INTO search_index (entity, entity_id, profile_id, work_id, body)
    VALUES ('fact', NEW.id, NEW.profile_id, NULL,
        NEW.body || ' ' || coalesce(NEW.when_label, '') || ' ' || coalesce(NEW.source_label, ''));
END;

CREATE TRIGGER search_fact_on_delete AFTER DELETE ON canon_fact
BEGIN
    DELETE FROM search_index WHERE entity = 'fact' AND entity_id = OLD.id;
END;
