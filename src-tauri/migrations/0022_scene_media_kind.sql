-- A scene's material is a list with a kind: the still, and the video cut from
-- it.
--
-- 0021 gave a scene its frames — several candidates, an order, at most one
-- chosen. The video of a scene has exactly that life: a generator answers an
-- animation prompt with more than one take, comparing them is the work, and
-- one of them is what the montage uses. Everything a frame needed, a video
-- needs, and it needs it in the same list: the montage list reads one row per
-- scene with its still AND its clip, which across two tables would be a join
-- written at every call site rather than an order-by.
--
-- So the table keeps its name and widens its meaning. `kind` says what a row
-- is; `frame` is what every row already is, which is why the column arrives
-- with that default and no backfill is needed.
--
-- The chosen-one index widens with it. "At most one chosen per scene" becomes
-- "at most one chosen per kind per scene": a scene with its still picked AND
-- its clip picked is the ordinary end state, not a collision. The old index
-- would have made it unrepresentable, which is the right instinct pointed at
-- the wrong pair.

ALTER TABLE scene_frame ADD COLUMN kind TEXT NOT NULL DEFAULT 'frame';

-- The uniqueness that was already there, per kind. A picture and a clip of
-- the same scene are different rows of different kinds, so the old
-- `UNIQUE (scene_id, asset_id)` is untouched and still right: one asset is
-- hung on a scene once.
DROP INDEX scene_frame_one_selected;

CREATE UNIQUE INDEX scene_frame_one_selected ON scene_frame (scene_id, kind)
WHERE is_selected = 1;

-- The board reads a scene's material by kind, in order.
DROP INDEX scene_frame_scene;

CREATE INDEX scene_frame_scene ON scene_frame (scene_id, kind, position);
