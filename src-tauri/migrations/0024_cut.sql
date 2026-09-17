-- A short cut out of a longer work: which stretches of the donor it is, in
-- what order.
--
-- A short is not a new kind of thing. The `short` kind has stood in the
-- profile since v0.60 with its own axes, roles and release kind, and what
-- tells "cut out of a finished video" from "shot for itself" is whether it
-- has a donor — a `work_link` of role `donor`, which 0015 already holds. One
-- model, the donor being the whole of the difference. What is missing is
-- where in the donor.
--
-- ## Why a table, and not two columns on `work`
--
-- Because one short is routinely spliced from several stretches of one
-- video: the line that lands, then the chorus eight bars later. A `cut_from`
-- and a `cut_to` on the work can say the first and cannot say the second,
-- and a person who needs the second gets no warning — they get a product
-- that quietly cannot hold what they are making. The boundaries are a list
-- by nature, so they are a list here: rows in order, each a stretch, and a
-- short that is one unbroken stretch is that list with one row in it.
--
-- ## Why not `scene`
--
-- A scene (0016) also carries `starts_at` and `ends_at`, and the temptation
-- to reuse it is real. It is the wrong table. A scene's seconds say where it
-- plays in *its own* video — it is a row of a storyboard, a prompt and a
-- frame. A cut's seconds say where it was taken from in *someone else's*.
-- Putting both in one table makes the meaning of a row depend on whether the
-- work has a donor, which is two truths in one place (ADR 0001). A short can
-- have both at once, and legitimately: cut from a video *and* storyboarded
-- for its titles.
--
-- ## Where the seconds are measured
--
-- On the donor's timeline, from its start, in seconds — the same unit
-- `work.meta.duration` took in 0018, so a cut can be drawn against the
-- donor's length without a conversion anywhere. The end is exclusive of
-- nothing and inclusive of nothing: it is a mark on a line, and a cut of
-- 12.0–15.5 is three and a half seconds of video. Sub-second precision is
-- the point of REAL here — a cut that lands a frame late is a cut that
-- starts on the wrong word.
--
-- ## What the core does with them
--
-- Nothing but keep them and hand them out. Cutting the file belongs to the
-- plugin of v1.10 (decision of 2026-09-11: no ffmpeg in the core), and what
-- the plugin is given is this list joined to the donor's video asset — the
-- paths and the boundaries. The core stays a record of intent.

CREATE TABLE cut (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profile (id) ON DELETE CASCADE,
    -- The short this stretch is part of.
    work_id TEXT NOT NULL REFERENCES work (id) ON DELETE CASCADE,
    -- The work it is taken out of. Denormalised from the donor link on
    -- purpose: a short may be spliced from two different donors, so the
    -- source belongs to the stretch and not to the work. The link in
    -- `work_link` remains the statement "this was made from that", with the
    -- version it was taken at; this column is which of them this stretch is
    -- from.
    source_id TEXT NOT NULL REFERENCES work (id) ON DELETE CASCADE,
    -- Seconds on the source's timeline, from its start.
    starts_at REAL NOT NULL,
    ends_at REAL NOT NULL,
    -- The stretch's place in the splice, from 1. Plain integer with no
    -- UNIQUE, for the reason 0016 gives about scene numbers: a row restored
    -- from the trash must not be refused because its number was taken while
    -- it was gone.
    position INTEGER NOT NULL,
    -- What the person calls this stretch — "the hook", "the last line".
    -- Free text, and ordinarily empty: the picture on the track says more
    -- than a name does.
    label TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    -- A stretch that ends before it starts is not a short cut oddly, it is a
    -- typo. Zero length is refused for the same reason: it produces no video
    -- and no error anywhere downstream.
    CHECK (ends_at > starts_at),
    CHECK (starts_at >= 0),
    -- A work cut out of itself is a loop with no bottom.
    CHECK (work_id <> source_id)
);

CREATE INDEX cut_work ON cut (work_id, position);

-- "What has been cut out of this video already" — the question the donor's
-- own card asks, and the one the layout rule below asks of every short.
CREATE INDEX cut_source ON cut (source_id);

-- Tombstones, the shape 0011 gives every table a person's work lives in: a
-- cut deleted here must not read as a cut that never arrived here.
CREATE TRIGGER cut_tombstone_on_delete AFTER DELETE ON cut
BEGIN
    INSERT INTO tombstone (entity, entity_id, profile_id, label, deleted_at, deleted_by)
    VALUES ('cut', OLD.id, OLD.profile_id, coalesce(OLD.label, 'cut ' || OLD.position), strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device))
    ON CONFLICT (entity, entity_id) DO UPDATE SET
        profile_id = excluded.profile_id, label = excluded.label,
        deleted_at = excluded.deleted_at, deleted_by = excluded.deleted_by,
        restored_at = NULL, restored_by = NULL;
    DELETE FROM field_clock WHERE entity = 'cut' AND entity_id = OLD.id;
END;

CREATE TRIGGER cut_tombstone_on_insert AFTER INSERT ON cut
BEGIN
    UPDATE tombstone SET restored_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), restored_by = (SELECT id FROM device)
    WHERE entity = 'cut' AND entity_id = NEW.id AND restored_at IS NULL;
END;

-- Field clocks, so a cut edited on one machine and on another settles the
-- way every other edit settles (0011).
CREATE TRIGGER cut_clock_on_update AFTER UPDATE ON cut
BEGIN
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'cut', NEW.id, 'source_id', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.source_id IS NOT NEW.source_id
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'cut', NEW.id, 'starts_at', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.starts_at IS NOT NEW.starts_at
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'cut', NEW.id, 'ends_at', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.ends_at IS NOT NEW.ends_at
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'cut', NEW.id, 'position', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.position IS NOT NEW.position
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'cut', NEW.id, 'label', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.label IS NOT NEW.label
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
END;

-- ## The cover prompt
--
-- A short is picked from a wall of thumbnails, so its cover is not an
-- afterthought — it is written, kept and reworked like the video is. The
-- prompt has parts that are edited and copied separately: what to draw, what
-- to keep out, what words go on it.
--
-- That is the shape `scene.blocks` already has (0016): a JSON object keyed
-- by the kind's own vocabulary, each block a text a person edits on its own.
-- The same shape is used here rather than three columns, for the same
-- reason — the craft names its parts, the code does not know them (ADR
-- 0001). A craft whose covers carry a fourth part says so in its profile and
-- nothing here changes.
--
-- It is a column on `work` and not rows in `scene`, because a work has one
-- cover and a storyboard has many scenes; and it is not `work.meta`, because
-- meta is the row of craft *fields* shown under the title — a prompt is a
-- body, not a field, and putting a paragraph in the field row would break
-- the row as surely as it would hide the prompt.
--
-- The pictures made from the prompt need no new home: `asset` has held
-- `kind = 'cover'` since 0001, with the work it belongs to.
ALTER TABLE work ADD COLUMN cover TEXT NOT NULL DEFAULT '{}';

-- The clock trigger names its columns one by one, so a new column on a
-- clocked table is invisible to synchronisation until it is rewritten
-- (the lesson 0023 wrote down). Dropped and recreated whole.
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
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'work', NEW.id, 'cover', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.cover IS NOT NEW.cover
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
END;

-- The cover's prompt is text about the work, so it is text the search should
-- find: a person who remembers "the one with the neon corridor" remembers
-- the prompt, not the title. The work triggers of 0023 are rewritten to take
-- it in, and the index is refilled for the works already here.
DROP TRIGGER search_work_on_insert;
CREATE TRIGGER search_work_on_insert AFTER INSERT ON work
BEGIN
    INSERT INTO search_index (entity, entity_id, profile_id, work_id, body)
    VALUES ('work', NEW.id, NEW.profile_id, NEW.id,
        NEW.title || ' ' ||
        coalesce((SELECT group_concat(value, ' ') FROM json_each(NEW.meta)
                   WHERE type IN ('text', 'integer', 'real')), '') || ' ' ||
        coalesce((SELECT group_concat(value, ' ') FROM json_each(NEW.tags)), '') || ' ' ||
        coalesce((SELECT group_concat(value, ' ') FROM json_each(NEW.cover)
                   WHERE type = 'text'), ''));
END;

DROP TRIGGER search_work_on_update;
CREATE TRIGGER search_work_on_update AFTER UPDATE ON work
WHEN OLD.title IS NOT NEW.title OR OLD.meta IS NOT NEW.meta OR OLD.tags IS NOT NEW.tags
     OR OLD.cover IS NOT NEW.cover
BEGIN
    DELETE FROM search_index WHERE entity = 'work' AND entity_id = NEW.id;
    INSERT INTO search_index (entity, entity_id, profile_id, work_id, body)
    VALUES ('work', NEW.id, NEW.profile_id, NEW.id,
        NEW.title || ' ' ||
        coalesce((SELECT group_concat(value, ' ') FROM json_each(NEW.meta)
                   WHERE type IN ('text', 'integer', 'real')), '') || ' ' ||
        coalesce((SELECT group_concat(value, ' ') FROM json_each(NEW.tags)), '') || ' ' ||
        coalesce((SELECT group_concat(value, ' ') FROM json_each(NEW.cover)
                   WHERE type = 'text'), ''));
END;
