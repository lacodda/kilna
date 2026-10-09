-- The board of trials of an experiment (v0.95, ADR 0061).
--
-- An experiment is a work of a kind the profile marks as a lab: a brief, the
-- findings, the fields that hold it on course. What it is made of is trials:
-- a text tried out somewhere outside - a style prompt run through a music
-- generator, the opening of a chapter read aloud - listened to, and kept or
-- thrown away. A trial is a row, an experiment has dozens, and each is judged
-- on its own: the shape a cover's board of ideas already has (0034).
--
-- ## Why rows, and not versions of the experiment
--
-- Versions are the history of one text; trials are rivals. A field of twelve
-- trials sent out at once is not twelve revisions of anything, and "keep this
-- one, drop that one" is not a property a version has. Nor are they versions
-- of the song they may end in: a sweep of the field belongs to no song yet.
--
-- ## Series and lineage
--
-- `series` is how the person grouped them: "a sweep of the field", "around
-- the core", "a rework: <song>". Free words, because the series are the
-- experiment's own. `parent_id` is the trial this one varies - recorded where
-- the variation is made, never guessed (ADR 0055) - and `angle` says what was
-- moved. `position` orders a series.
--
-- ## The body and where it came from
--
-- `body` is the text, and it is the truth. `bricks` lists the dictionary's
-- bricks it was picked from, when it was: provenance, not a second copy of
-- the text - edit the body and it says what it says. `reference` is who to
-- listen to, which never goes into the body (a generator ignores band names,
-- or refuses them). `source_version_id` is the song's style a rework starts
-- from.
--
-- ## After it was heard
--
-- `outcome` is what came out, in the person's words, with the generator's
-- links. `verdict` is one column with two words - kept, dropped - or none
-- for not judged: a kept trial that is also dropped is not a state the board
-- can show (ADR 0050). `run_first` marks where a run starts.
--
-- What a kept trial became is recorded where it went (`work_version.trial_id`,
-- `style_brick.trial_id` below), never here: material is spent, not moved
-- (ADR 0045), and the trial finds its harvest by looking.

CREATE TABLE trial (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profile (id) ON DELETE CASCADE,
    -- The experiment whose board it is on. Goes with the work to the trash
    -- and comes back with it (see `trash::cascade`).
    work_id TEXT NOT NULL REFERENCES work (id) ON DELETE CASCADE,
    series TEXT NOT NULL DEFAULT '',
    position INTEGER NOT NULL DEFAULT 0,
    parent_id TEXT REFERENCES trial (id) ON DELETE SET NULL,
    angle TEXT NOT NULL DEFAULT '',
    body TEXT NOT NULL DEFAULT '',
    bricks TEXT NOT NULL DEFAULT '[]',
    reference TEXT NOT NULL DEFAULT '',
    outcome TEXT NOT NULL DEFAULT '',
    verdict TEXT,
    source_version_id TEXT REFERENCES work_version (id) ON DELETE SET NULL,
    run_first INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (verdict IS NULL OR verdict IN ('keep', 'drop')),
    CHECK (run_first IN (0, 1)),
    CHECK (json_valid(bricks) AND json_type(bricks) = 'array')
);

-- A board, series by series, in order.
CREATE INDEX trial_work ON trial (work_id, series, position);
CREATE INDEX trial_parent ON trial (parent_id);
CREATE INDEX trial_source ON trial (source_version_id);

-- What a kept trial became. A version of a song's style taken from it, a
-- phrase of the dictionary cut from it: each remembers the trial, and the
-- trial shows where it went by asking. Set to nothing when the trial goes,
-- and pointed back when it is restored (`trash::SPENT`).
ALTER TABLE work_version ADD COLUMN trial_id TEXT REFERENCES trial (id) ON DELETE SET NULL;
CREATE INDEX work_version_trial ON work_version (trial_id);

ALTER TABLE style_brick ADD COLUMN trial_id TEXT REFERENCES trial (id) ON DELETE SET NULL;
CREATE INDEX style_brick_trial ON style_brick (trial_id);

-- The takes a generator gave for a trial - the sounds it is judged by - are
-- files of the trial, the way a brick's references are files of the brick
-- (0026). They go with it to the trash and come back with it.
ALTER TABLE asset ADD COLUMN trial_id TEXT REFERENCES trial (id) ON DELETE CASCADE;
CREATE INDEX asset_trial ON asset (trial_id);

-- Tombstones, the shape 0011 gives every table a person's work lives in. The
-- label is the angle, or the body's opening words when it has none.
CREATE TRIGGER trial_tombstone_on_delete AFTER DELETE ON trial
BEGIN
    INSERT INTO tombstone (entity, entity_id, profile_id, label, deleted_at, deleted_by)
    VALUES ('trial', OLD.id, OLD.profile_id,
        substr(coalesce(nullif(trim(OLD.angle), ''), OLD.body), 1, 80),
        strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device))
    ON CONFLICT (entity, entity_id) DO UPDATE SET
        profile_id = excluded.profile_id, label = excluded.label,
        deleted_at = excluded.deleted_at, deleted_by = excluded.deleted_by,
        restored_at = NULL, restored_by = NULL;
    DELETE FROM field_clock WHERE entity = 'trial' AND entity_id = OLD.id;
END;

CREATE TRIGGER trial_tombstone_on_insert AFTER INSERT ON trial
BEGIN
    UPDATE tombstone SET restored_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), restored_by = (SELECT id FROM device)
    WHERE entity = 'trial' AND entity_id = NEW.id AND restored_at IS NULL;
END;

-- Field clocks, so a trial kept on one machine and heard again on another
-- settles the way every other edit settles (0011). One statement per column:
-- a column left out here is invisible to synchronisation (the lesson of 0023).
CREATE TRIGGER trial_clock_on_update AFTER UPDATE ON trial
BEGIN
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'trial', NEW.id, 'series', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.series IS NOT NEW.series
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'trial', NEW.id, 'position', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.position IS NOT NEW.position
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'trial', NEW.id, 'parent_id', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.parent_id IS NOT NEW.parent_id
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'trial', NEW.id, 'angle', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.angle IS NOT NEW.angle
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'trial', NEW.id, 'body', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.body IS NOT NEW.body
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'trial', NEW.id, 'bricks', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.bricks IS NOT NEW.bricks
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'trial', NEW.id, 'reference', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.reference IS NOT NEW.reference
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'trial', NEW.id, 'outcome', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.outcome IS NOT NEW.outcome
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'trial', NEW.id, 'verdict', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.verdict IS NOT NEW.verdict
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'trial', NEW.id, 'source_version_id', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.source_version_id IS NOT NEW.source_version_id
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'trial', NEW.id, 'run_first', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.run_first IS NOT NEW.run_first
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
END;
