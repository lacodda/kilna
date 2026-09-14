-- A scene points at the notes it is about: who is in it, where it happens.
--
-- "Every scene with her in it" is the question a storyboard has to answer,
-- and free text in the description cannot: a hero named three ways across
-- fifty scenes is three heroes to anything that reads them. So a scene holds
-- references, and the thing it references is a note of a kind the profile
-- names -- `character`, `location` (decision of 2026-09-11: one entity, two
-- stages; the character's own editing screen is v0.73, and it edits the note
-- this points at rather than a second row meaning the same person).
--
-- A row per (scene, note): a scene has several people and places in it, and a
-- person is in many scenes. No role column -- what a note *is* is its kind,
-- and a second word for it here would be a place for the two to disagree.
--
-- The kind is not checked in the schema: a profile's vocabulary is the
-- profile's, it changes while the workspace lives, and a database that
-- refused a note whose kind was renamed yesterday would lose the link rather
-- than the word. The application checks it on write, the way it checks a
-- scene's kind of shot.

CREATE TABLE scene_note (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profile (id) ON DELETE CASCADE,
    scene_id TEXT NOT NULL REFERENCES scene (id) ON DELETE CASCADE,
    note_id TEXT NOT NULL REFERENCES note (id) ON DELETE CASCADE,
    created_at TEXT NOT NULL,
    UNIQUE (scene_id, note_id)
);

CREATE INDEX scene_note_note ON scene_note (note_id);

-- Tombstones, the shape 0011 gives every table a person's work lives in: a
-- link deleted here must not read as a link that never arrived here.
CREATE TRIGGER scene_note_tombstone_on_delete AFTER DELETE ON scene_note
BEGIN
    INSERT INTO tombstone (entity, entity_id, profile_id, label, deleted_at, deleted_by)
    VALUES ('scene_note', OLD.id, OLD.profile_id, (SELECT title FROM note WHERE id = OLD.note_id), strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device))
    ON CONFLICT (entity, entity_id) DO UPDATE SET
        profile_id = excluded.profile_id, label = excluded.label,
        deleted_at = excluded.deleted_at, deleted_by = excluded.deleted_by,
        restored_at = NULL, restored_by = NULL;
END;

CREATE TRIGGER scene_note_tombstone_on_insert AFTER INSERT ON scene_note
BEGIN
    UPDATE tombstone SET restored_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), restored_by = (SELECT id FROM device)
    WHERE entity = 'scene_note' AND entity_id = NEW.id AND restored_at IS NULL;
END;
