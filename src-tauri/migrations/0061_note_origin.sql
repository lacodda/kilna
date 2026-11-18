-- The work a note arose at, apart from the work it was spent in (v0.145,
-- FR-09 9.3): a phrase that came while a song was written stays fresh and
-- remembers the song. `work_id` keeps saying where it was used.
ALTER TABLE note ADD COLUMN origin_work_id TEXT REFERENCES work(id) ON DELETE SET NULL;
CREATE INDEX note_origin_work ON note(origin_work_id) WHERE origin_work_id IS NOT NULL;
