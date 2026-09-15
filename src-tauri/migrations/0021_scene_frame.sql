-- A scene keeps the frames drawn for it, in order, and says which one is it.
--
-- A generator answers a prompt with four pictures, not one. The board needs
-- all four -- comparing them is the work -- and it needs to know which one
-- the video will be cut from. So a frame is a row, a scene has several, and
-- one of them is the scene's own.
--
-- This is its own table rather than columns on `asset`. The asset table
-- (0001) hangs on `work_id` / `release_id`; a scene is not in it at all, and
-- a frame also needs an order and a mark for the chosen one. Three columns
-- that stand empty on every cover and every attachment would be three
-- columns saying that one table holds two different lives. The file itself
-- stays where 0027 put it -- copied into `media/`, named by its id, its
-- arrival name remembered -- and this table says only whose frame it is and
-- where it stands.
--
-- The shape is `scene_note`'s (0019): a profile, references that cascade,
-- tombstones on both ends. What it adds is what a link between a scene and a
-- note never needed: `position`, the way `scene` itself numbers frames from
-- 1, and `is_selected`.
--
-- Two frames chosen for one scene is not a state the schema allows: the
-- partial unique index below makes it unrepresentable rather than something
-- the application has to remember to check. A scene with no frame chosen is
-- allowed and ordinary -- four candidates and no verdict yet is the normal
-- middle of the work.
--
-- Deleting the asset deletes the frame: a frame whose bytes are gone is a
-- broken picture, not a record of anything (ADR 0027 says the same about
-- detaching, and for the same reason).

CREATE TABLE scene_frame (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profile (id) ON DELETE CASCADE,
    scene_id TEXT NOT NULL REFERENCES scene (id) ON DELETE CASCADE,
    asset_id TEXT NOT NULL REFERENCES asset (id) ON DELETE CASCADE,
    -- The frame's place among the scene's frames, from 1.
    position INTEGER NOT NULL,
    -- The one the video is cut from. At most one per scene; see the index.
    is_selected INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    UNIQUE (scene_id, asset_id)
);

CREATE INDEX scene_frame_scene ON scene_frame (scene_id, position);
CREATE INDEX scene_frame_asset ON scene_frame (asset_id);

-- One chosen frame per scene, or none. Partial index: the rows that are not
-- chosen are not in it, so they do not collide with each other.
CREATE UNIQUE INDEX scene_frame_one_selected ON scene_frame (scene_id)
WHERE is_selected = 1;

-- Tombstones, the shape 0011 gives every table a person's work lives in: a
-- frame deleted here must not read as a frame that never arrived here. The
-- label is the name the file came in under, which is what the person would
-- recognise in a list of what was removed.
CREATE TRIGGER scene_frame_tombstone_on_delete AFTER DELETE ON scene_frame
BEGIN
    INSERT INTO tombstone (entity, entity_id, profile_id, label, deleted_at, deleted_by)
    VALUES ('scene_frame', OLD.id, OLD.profile_id, (SELECT original_name FROM asset WHERE id = OLD.asset_id), strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device))
    ON CONFLICT (entity, entity_id) DO UPDATE SET
        profile_id = excluded.profile_id, label = excluded.label,
        deleted_at = excluded.deleted_at, deleted_by = excluded.deleted_by,
        restored_at = NULL, restored_by = NULL;
END;

CREATE TRIGGER scene_frame_tombstone_on_insert AFTER INSERT ON scene_frame
BEGIN
    UPDATE tombstone SET restored_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), restored_by = (SELECT id FROM device)
    WHERE entity = 'scene_frame' AND entity_id = NEW.id AND restored_at IS NULL;
END;
