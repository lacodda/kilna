-- The text a score judged (v0.170, FR-01): the body its version had the
-- moment the score was written. A version is edited in place, so without it
-- nothing says how far the text has moved since - and a typo mended after a
-- restart was scored again as if the song had been rewritten. Kept as the
-- text rather than a hash of it: "has it changed enough" is a measure of two
-- texts, and a hash only answers "is it the same". Scores written before
-- have none and are measured against nothing, as they were.
ALTER TABLE work_score ADD COLUMN judged_text TEXT;
