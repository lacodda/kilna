-- What was heard in a donor's finished file (v0.103, ADR 0071).
--
-- A short is cut from a video, and a person deciding where to cut listens
-- for where the video's parts begin and which of them carries the song. A
-- plugin can hear both - kilna never opens the file itself - and this table
-- keeps what it heard: the seconds where the parts begin, for the track to
-- draw and a dragged edge to land on, and the parts worth a short, each with
-- its reason.
--
-- What was heard belongs to a file, not to the work: it is kept with the
-- file's path, size and moment it was last written, and read only while the
-- donor's finished file is still that file. A new render is a new file, and
-- what was heard in the old one stops being offered rather than standing
-- over sound that is no longer there.
--
-- So this is a record of a file, like the media folder is a record of a
-- machine: no field clocks, no tombstone, never logged, merged or exported.
-- Hearing it again rebuilds it; a replay has nothing to rebuild it from.

CREATE TABLE donor_map (
    work_id TEXT PRIMARY KEY REFERENCES work (id) ON DELETE CASCADE,
    profile_id TEXT NOT NULL REFERENCES profile (id) ON DELETE CASCADE,
    -- The file that was heard, whole, and how it stood then.
    file TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    file_modified TEXT NOT NULL,
    -- Seconds where its parts begin, as a JSON array of numbers.
    boundaries TEXT NOT NULL,
    -- The parts offered, best first, as a JSON array of
    -- {starts_at, ends_at, reason, score}.
    parts TEXT NOT NULL,
    -- The plugin that heard it, and when.
    heard_by TEXT NOT NULL,
    heard_at TEXT NOT NULL
);
