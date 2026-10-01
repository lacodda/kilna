-- The board of ideas for a publication's cover (v0.89).
--
-- A cover is one concept per publication (`work.cover`, ADR 0049). Before a
-- person settles on it they look at several: their own idea, the same idea
-- worked out by the assistant, a handful the assistant proposes from other
-- angles, the cover a neighbouring publication of the same song already has.
-- Each of those is a concept of the same shape as the cover, and each is
-- judged on its own - kept on the shortlist, turned down, taken into the
-- constructor. So an idea is a row, a publication has several, and the
-- cover stays the one record it was.
--
-- ## Why a table, and not a list inside `work.cover`
--
-- A list inside the cover would make every star a rewrite of the whole
-- cover, an undo of a star would take back whatever else changed in it, and
-- two machines marking two ideas would collide on one field. Ideas are many,
-- marked one by one, and go to the trash one by one: the shape a scene and a
-- comment already have.
--
-- ## Where it came from
--
-- `source` is one of four words. `own` is the person's idea in their words;
-- `refined` is that idea worked out by the assistant; `ai` is the
-- assistant's own; `sibling` is a neighbouring publication's cover, copied
-- when it was starred here, with `from_work_id` naming the publication. A
-- neighbour deleted for good leaves the copy and drops the name.
--
-- ## One verdict, or none
--
-- On the shortlist, turned down, or not judged yet. One column with two
-- words rather than two flags: a starred idea that is also turned down is
-- not a state the board can show, so it is not one the schema can hold.
--
-- ## The concept is the cover's shape
--
-- `concept` is a cover as `work.cover` holds it, read by the same struct. An
-- idea taken into the constructor is copied into the cover; nothing points
-- back. The angle - what sets it apart from the others on the board - and
-- the headline it is shown under are the idea's own, not the cover's.
--
-- Not in the search: an idea is a candidate. The cover that was chosen is
-- found through `work.cover`.

CREATE TABLE cover_idea (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profile (id) ON DELETE CASCADE,
    -- The publication whose board it is on. Goes with the work to the trash
    -- and comes back with it (see `trash::cascade`).
    work_id TEXT NOT NULL REFERENCES work (id) ON DELETE CASCADE,
    source TEXT NOT NULL,
    -- The publication a `sibling` idea was copied from.
    from_work_id TEXT REFERENCES work (id) ON DELETE SET NULL,
    -- What sets it apart: "a close-up from below", "the hero small in the yard".
    angle TEXT NOT NULL DEFAULT '',
    -- What the card is called on the board.
    headline TEXT NOT NULL DEFAULT '',
    -- A cover, as `work.cover` holds it.
    concept TEXT NOT NULL DEFAULT '{}',
    verdict TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (source IN ('own', 'refined', 'ai', 'sibling')),
    CHECK (verdict IS NULL OR verdict IN ('star', 'rejected')),
    CHECK (json_valid(concept))
);

-- A publication's board, oldest first.
CREATE INDEX cover_idea_work ON cover_idea (work_id, created_at);
CREATE INDEX cover_idea_from ON cover_idea (from_work_id);

-- Tombstones, the shape 0011 gives every table a person's work lives in. The
-- label is the headline, or the idea's words when it has none.
CREATE TRIGGER cover_idea_tombstone_on_delete AFTER DELETE ON cover_idea
BEGIN
    INSERT INTO tombstone (entity, entity_id, profile_id, label, deleted_at, deleted_by)
    VALUES ('cover_idea', OLD.id, OLD.profile_id,
        substr(coalesce(nullif(trim(OLD.headline), ''), json_extract(OLD.concept, '$.idea'), ''), 1, 80),
        strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device))
    ON CONFLICT (entity, entity_id) DO UPDATE SET
        profile_id = excluded.profile_id, label = excluded.label,
        deleted_at = excluded.deleted_at, deleted_by = excluded.deleted_by,
        restored_at = NULL, restored_by = NULL;
    DELETE FROM field_clock WHERE entity = 'cover_idea' AND entity_id = OLD.id;
END;

CREATE TRIGGER cover_idea_tombstone_on_insert AFTER INSERT ON cover_idea
BEGIN
    UPDATE tombstone SET restored_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), restored_by = (SELECT id FROM device)
    WHERE entity = 'cover_idea' AND entity_id = NEW.id AND restored_at IS NULL;
END;

-- Field clocks, so an idea starred on one machine and turned down on another
-- settles the way every other edit settles (0011). One statement per column:
-- a column left out here is invisible to synchronisation (the lesson of 0023).
CREATE TRIGGER cover_idea_clock_on_update AFTER UPDATE ON cover_idea
BEGIN
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'cover_idea', NEW.id, 'source', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.source IS NOT NEW.source
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'cover_idea', NEW.id, 'from_work_id', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.from_work_id IS NOT NEW.from_work_id
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'cover_idea', NEW.id, 'angle', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.angle IS NOT NEW.angle
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'cover_idea', NEW.id, 'headline', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.headline IS NOT NEW.headline
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'cover_idea', NEW.id, 'concept', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.concept IS NOT NEW.concept
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'cover_idea', NEW.id, 'verdict', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.verdict IS NOT NEW.verdict
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
END;
