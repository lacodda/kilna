-- A word written down once, and a publication that goes out once (v0.90).
--
-- Runs with foreign keys off (`Migration::rebuilds`): the term table is
-- written anew, because SQLite cannot loosen a column's constraint in place.
-- The runner checks every key before the step commits.
--
-- ## One publication, one release (ADR 0051)
--
-- A publication is what goes out: a clip on a video platform, the track
-- under a picture, a short. Two places are two publications, each with its
-- own cover in its own shape, its own comments, its own numbers - and so a
-- release belongs to one work, and a work has at most one. The index below
-- says so; until now it was an agreement the owner's workspace happened to
-- keep (193 publications, 193 releases, 2026-10-01).
--
-- A workspace that holds two releases on one work is carried over without
-- loss: the work keeps its first release, and each other one becomes a
-- publication of its own - the same kind, the fields, the cover and the
-- frame of the work it leaves, made from what that work was made from, and
-- its files with it. Its title is provisional - the work's and the door's -
-- until the naming step that runs when the workspace opens (`naming`)
-- writes it the way kilna names what it makes.

CREATE TEMP TABLE release_split (
    release_id TEXT PRIMARY KEY,
    work_id TEXT NOT NULL,
    new_work_id TEXT NOT NULL
);

-- Every release but the first of its work. A version 4 id, as `Minted`
-- writes them.
INSERT INTO release_split (release_id, work_id, new_work_id)
SELECT r.id, r.work_id,
       lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2)
             || '-' || substr('89ab', 1 + (abs(random()) % 4), 1) || substr(hex(randomblob(2)), 2)
             || '-' || hex(randomblob(6)))
  FROM release r
 WHERE EXISTS (
       SELECT 1 FROM release first
        WHERE first.work_id = r.work_id
          AND (first.created_at < r.created_at
               OR (first.created_at = r.created_at AND first.rowid < r.rowid)));

INSERT INTO work (id, profile_id, collection_id, kind, title, status, meta, current_version_id,
                  position, created_at, updated_at, status_pinned_at, tags, marks, tier_pinned,
                  tier_pinned_at, tier_pin_reason, bookmarked_at, stage, cover, frame)
SELECT s.new_work_id, w.profile_id, w.collection_id, w.kind, w.title || ' (' || r.kind || ')',
       w.status, w.meta, NULL, w.position, r.created_at, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
       w.status_pinned_at, w.tags, w.marks, NULL, NULL, NULL, NULL, w.stage, w.cover, w.frame
  FROM release_split s
  JOIN work w ON w.id = s.work_id
  JOIN release r ON r.id = s.release_id;

-- Made from what the work it leaves was made from.
INSERT INTO work_link (id, profile_id, work_id, source_id, role, source_version_id, created_at)
SELECT lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)), 2)
             || '-' || substr('89ab', 1 + (abs(random()) % 4), 1) || substr(hex(randomblob(2)), 2)
             || '-' || hex(randomblob(6))),
       l.profile_id, s.new_work_id, l.source_id, l.role, l.source_version_id, l.created_at
  FROM release_split s
  JOIN work_link l ON l.work_id = s.work_id;

UPDATE asset SET work_id = (SELECT new_work_id FROM release_split WHERE release_id = asset.release_id)
 WHERE release_id IN (SELECT release_id FROM release_split);

UPDATE release SET work_id = (SELECT new_work_id FROM release_split WHERE release_id = release.id)
 WHERE id IN (SELECT release_id FROM release_split);

DROP TABLE release_split;

DROP INDEX release_work;
CREATE UNIQUE INDEX release_one_per_work ON release (work_id);

-- ## The release's own title goes
--
-- What a publication is called is the work's title; what it goes out under
-- is a field of its door (`meta.title`), which the window shows, the
-- assistant writes and the platform receives. `release.title` was a third
-- word for the same thing, written by the first imports and read by no
-- screen since the fields arrived. Where the field is empty the column's
-- word moves into it; where the field says something else the old word is
-- kept beside it, under `former_title`, rather than thrown away.
UPDATE release
   SET meta = json_set(meta, '$.title', title)
 WHERE title IS NOT NULL AND trim(title) <> ''
   AND coalesce(trim(json_extract(meta, '$.title')), '') = '';

UPDATE release
   SET meta = json_set(meta, '$.former_title', title)
 WHERE title IS NOT NULL AND trim(title) <> ''
   AND json_extract(meta, '$.title') IS NOT title;

-- The triggers that read the column go first: a column a trigger names
-- cannot be dropped.
DROP TRIGGER release_tombstone_on_delete;
DROP TRIGGER release_clock_on_update;

ALTER TABLE release DROP COLUMN title;

-- A release's label in the trash is its work's title and its door, the way
-- the window names it.
CREATE TRIGGER release_tombstone_on_delete AFTER DELETE ON release
BEGIN
    INSERT INTO tombstone (entity, entity_id, profile_id, label, deleted_at, deleted_by)
    VALUES ('release', OLD.id, NULL,
            coalesce((SELECT title FROM work WHERE id = OLD.work_id) || ' · ' || OLD.kind, OLD.kind),
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device))
    ON CONFLICT (entity, entity_id) DO UPDATE SET
        profile_id = excluded.profile_id, label = excluded.label,
        deleted_at = excluded.deleted_at, deleted_by = excluded.deleted_by,
        restored_at = NULL, restored_by = NULL;
    DELETE FROM field_clock WHERE entity = 'release' AND entity_id = OLD.id;
END;

CREATE TRIGGER release_clock_on_update AFTER UPDATE ON release
BEGIN
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'release', NEW.id, 'kind', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.kind IS NOT NEW.kind
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'release', NEW.id, 'status', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.status IS NOT NEW.status
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'release', NEW.id, 'scheduled_at', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.scheduled_at IS NOT NEW.scheduled_at
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'release', NEW.id, 'released_at', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.released_at IS NOT NEW.released_at
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'release', NEW.id, 'url', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.url IS NOT NEW.url
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'release', NEW.id, 'meta', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.meta IS NOT NEW.meta
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'release', NEW.id, 'slot_pinned_at', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.slot_pinned_at IS NOT NEW.slot_pinned_at
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'release', NEW.id, 'scheduled_time', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.scheduled_time IS NOT NEW.scheduled_time
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'release', NEW.id, 'time_zone', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.time_zone IS NOT NEW.time_zone
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
END;

-- ## One word, one record (ADR 0052)
--
-- A word the owner keeps for a song to come, a word the register says is
-- spent, and the way a word is sung are three facts about one word, not
-- three rows: "пульсар" collected in the bank and sung in a song that went
-- out is spent from that day, and the guard sees it without a second copy.
-- So a term gains facets, each optional:
--
-- `strictness` - in the register, and how strictly; none is a word that is
-- not spent at all. Loosened from NOT NULL, which is why the table is
-- rebuilt.
--
-- `bank` - in the bank of words, and where it stands there: fresh, set
-- aside, given up on. None is a word that is not collected. Where it was
-- sung is not stored: it is found in the texts, as the register's counts
-- are (ADR 0044).
--
-- `sung` - how its forms are sung where that is not how they are written:
-- a list of {written, sung} - "Марсель" sung "МарсЭль", "пульсар" sung
-- "пульсАр". The capital vowel is the stress, the way the owner writes it
-- for the singer; a changed letter is a respelling.
CREATE TABLE term_rebuilt (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profile (id) ON DELETE CASCADE,
    word TEXT NOT NULL,
    forms TEXT NOT NULL DEFAULT '[]',
    kind TEXT NOT NULL DEFAULT 'noun',
    strictness TEXT,
    bank TEXT,
    sung TEXT NOT NULL DEFAULT '[]',
    topic TEXT,
    note TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (trim(word) <> ''),
    CHECK (json_valid(forms) AND json_type(forms) = 'array'),
    CHECK (json_valid(sung) AND json_type(sung) = 'array'),
    CHECK (kind IN ('noun', 'adjective', 'verb', 'phrase', 'image', 'scene', 'pattern')),
    CHECK (strictness IS NULL OR strictness IN ('ban', 'limit', 'rare')),
    CHECK (bank IS NULL OR bank IN ('fresh', 'parked', 'dropped'))
) STRICT;

INSERT INTO term_rebuilt (id, profile_id, word, forms, kind, strictness, bank, sung, topic, note,
                          created_at, updated_at)
SELECT id, profile_id, word, forms, kind, strictness, NULL, '[]', topic, note, created_at, updated_at
  FROM term;

-- The trigger on the named works reads the term's word for its label; it
-- would name a table that is, for a moment, not there.
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
    SELECT 'term', NEW.id, 'bank', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.bank IS NOT NEW.bank
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

-- ## Blocks of words
--
-- The bank is sorted into blocks the owner names - "space", "the kitchen",
-- "for the slow one" - in an order they choose, and a word may stand in
-- several. A block is a row with a position; a word in a block is a row of
-- the pair, the shape a term and the works named for it have.
CREATE TABLE term_block (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profile (id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (trim(name) <> '')
) STRICT;

CREATE INDEX term_block_profile ON term_block (profile_id, position);

CREATE TABLE term_block_word (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profile (id) ON DELETE CASCADE,
    block_id TEXT NOT NULL REFERENCES term_block (id) ON DELETE CASCADE,
    term_id TEXT NOT NULL REFERENCES term (id) ON DELETE CASCADE,
    created_at TEXT NOT NULL,
    UNIQUE (block_id, term_id)
) STRICT;

CREATE INDEX term_block_word_term ON term_block_word (term_id);

CREATE TRIGGER term_block_tombstone_on_delete AFTER DELETE ON term_block
BEGIN
    INSERT INTO tombstone (entity, entity_id, profile_id, label, deleted_at, deleted_by)
    VALUES ('term_block', OLD.id, OLD.profile_id, OLD.name, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device))
    ON CONFLICT (entity, entity_id) DO UPDATE SET
        profile_id = excluded.profile_id, label = excluded.label,
        deleted_at = excluded.deleted_at, deleted_by = excluded.deleted_by,
        restored_at = NULL, restored_by = NULL;
    DELETE FROM field_clock WHERE entity = 'term_block' AND entity_id = OLD.id;
END;

CREATE TRIGGER term_block_tombstone_on_insert AFTER INSERT ON term_block
BEGIN
    UPDATE tombstone SET restored_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), restored_by = (SELECT id FROM device)
    WHERE entity = 'term_block' AND entity_id = NEW.id AND restored_at IS NULL;
END;

CREATE TRIGGER term_block_clock_on_update AFTER UPDATE ON term_block
BEGIN
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'term_block', NEW.id, 'name', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.name IS NOT NEW.name
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'term_block', NEW.id, 'position', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.position IS NOT NEW.position
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
END;

CREATE TRIGGER term_block_word_tombstone_on_delete AFTER DELETE ON term_block_word
BEGIN
    INSERT INTO tombstone (entity, entity_id, profile_id, label, deleted_at, deleted_by)
    VALUES ('term_block_word', OLD.id, OLD.profile_id, (SELECT word FROM term WHERE id = OLD.term_id), strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device))
    ON CONFLICT (entity, entity_id) DO UPDATE SET
        profile_id = excluded.profile_id, label = excluded.label,
        deleted_at = excluded.deleted_at, deleted_by = excluded.deleted_by,
        restored_at = NULL, restored_by = NULL;
END;

CREATE TRIGGER term_block_word_tombstone_on_insert AFTER INSERT ON term_block_word
BEGIN
    UPDATE tombstone SET restored_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), restored_by = (SELECT id FROM device)
    WHERE entity = 'term_block_word' AND entity_id = NEW.id AND restored_at IS NULL;
END;

-- ## A finding the owner has heard (ADR 0054)
--
-- The guard of repeats is derived, every time it is asked, from the texts
-- and the calendar; what is stored is a person's answer to one of its
-- findings - "I know, I am keeping it" - on the pair of a work and a word.
-- The word as the finding names it ("пульсар"), matched by its stem when
-- read back. No key on the work, like a dismissed complaint on the
-- dashboard (ADR 0010): an answer is a decision, not a fact about the row,
-- and it outlives a trip of the work to the trash and back.
CREATE TABLE repeat_kept (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profile (id) ON DELETE CASCADE,
    work_id TEXT NOT NULL,
    word TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE (work_id, word),
    CHECK (trim(word) <> '')
) STRICT;

CREATE INDEX repeat_kept_profile ON repeat_kept (profile_id);
