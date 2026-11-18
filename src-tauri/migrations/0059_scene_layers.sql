-- A scene's own layers over its clip's cover, and the movements of its shot
-- (v0.134, FR-07 7.2): the cover's bricks as JSON, and a JSON list of
-- motion brick ids. Empty for a scene drawn in the clip's style alone.
ALTER TABLE scene ADD COLUMN bricks TEXT;
ALTER TABLE scene ADD COLUMN motion_bricks TEXT;
