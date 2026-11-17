-- The gesture an operation belongs to (v0.114).
--
-- One thing a person asks for may be recorded as several operations: an
-- assistant's package is a work, its versions and its board, "make a clip
-- from this" is a work, a link and a release. They were one unit of work
-- since v0.83 and one transaction, but the log could not say so, and undo
-- took back only the last of them.
--
-- Every operation recorded inside one outermost unit carries the same id
-- here. Operations logged before this, and one recorded outside any unit,
-- carry none and stand alone.
ALTER TABLE operation ADD COLUMN gesture_id TEXT;

CREATE INDEX operation_gesture ON operation (gesture_id) WHERE gesture_id IS NOT NULL;
