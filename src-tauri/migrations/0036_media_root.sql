-- The folder on this machine where a workspace's media lives outside kilna
-- (v0.93, ADR 0057).
--
-- A work's folder on disk is found, not attached: the profile says how a
-- work's folder is named under the root (`work_kinds[].folder`), and this
-- table says where the root is. The two live apart on purpose. The template
-- is the craft's - how a person lays their disk out for songs and their
-- clips - and travels with the profile; the root is a place on one machine,
-- `D:\` here and `/Volumes/media` there, and a profile carrying it would be
-- wrong on the next device and would put a private path into every profile
-- a person exports.
--
-- So this is the machine's own record: no field clocks, no tombstone, never
-- merged and never exported. One root per workspace profile, because each
-- craft keeps its media in its own place.

CREATE TABLE media_root (
    profile_id TEXT PRIMARY KEY REFERENCES profile (id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
