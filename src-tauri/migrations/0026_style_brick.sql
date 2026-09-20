-- A style is a brick of the workspace's own dictionary, not a text on a work.
--
-- Until now the only style kilna knew was a version in the role `style`: a
-- finished prompt, written on one work, kept beside its lyrics. That is the
-- right home for a finished prompt and it stays. What it cannot be is the
-- vocabulary the prompt is built from.
--
-- A brick — «the render recipe of NeoAnime», «this character», «this
-- environment» — belongs to the workspace, not to a work. The same character
-- stands in thirty videos; written as a version it would be thirty texts, and
-- an edit would reach one of them. So it is a row here, pointed at by
-- whichever prompt wants it, and edited in one place.
--
-- The brick's *type* is not a column of values the code knows. It is a word of
-- the craft — image style, character, environment, typography, camera angle,
-- pose, layers, composition, look — and so it lives in the profile document
-- with the rest of the vocabulary (ADR 0001), keyed here by string. A type
-- carries a `hint`: what to describe when a brick is of that type. That hint is
-- what makes one dictionary richer than several flat ones — the same photograph
-- yields a render recipe under `image-style` and a person under `character`,
-- because the type says which question is being answered.
--
-- `status` is the brick's own life, not the work's: `draft` while it is only
-- references and a name, `ready` once it carries a description that can go into
-- a prompt, `dropped` when the owner retires it without losing what it was.
-- Only `ready` bricks are offered to the constructor: a draft is unfinished by
-- definition, and offering it silently is how a picker ends up full of things
-- nobody has touched.
--
-- References are assets (ADR 0027), attached by `style_brick_id`, so a picture
-- arrives the one way pictures arrive in this workspace — copied into `media/`,
-- named by its id, its arrival name remembered — and needs no second store.

CREATE TABLE style_brick (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profile (id) ON DELETE CASCADE,
    -- A key of the profile's `style_types`. A string, not a foreign key: the
    -- vocabulary is a document, and a brick written under a type the profile
    -- later drops still says what it was (the same rule scores follow for
    -- their axes, migration 0012).
    type_key TEXT NOT NULL,
    name TEXT NOT NULL,
    -- The vetted text that goes into a prompt verbatim. NULL while the brick
    -- is a draft of references with nothing written yet.
    description TEXT,
    -- The owner's steer, handed to the assistant when it describes the brick
    -- from the references: «only the jacket, ignore the background». Kept
    -- apart from the description because it is an instruction about the
    -- description, and it must never reach a prompt as if it were one.
    hint TEXT,
    status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'ready', 'dropped')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    -- Two bricks of one type may not share a name inside a workspace: the
    -- constructor shows «[Character] Ranger», and two of those are a picker
    -- the owner cannot choose from. Across types the same name is fine —
    -- a character and a look may both be «Ranger».
    UNIQUE (profile_id, type_key, name)
);

CREATE INDEX style_brick_profile ON style_brick (profile_id, type_key, status);

-- An asset can now belong to a brick, the way it already belongs to a work or
-- a release. Added rather than rebuilt: nothing points at a brick yet.
ALTER TABLE asset ADD COLUMN style_brick_id TEXT REFERENCES style_brick (id) ON DELETE CASCADE;

CREATE INDEX asset_style_brick ON asset (style_brick_id);

-- Tombstones, the shape 0011 gives every table a person's work lives in: a
-- brick deleted here must not read as a brick that never arrived. The label is
-- its name, which is what the owner would recognise in a list of what went.
CREATE TRIGGER style_brick_tombstone_on_delete AFTER DELETE ON style_brick
BEGIN
    INSERT INTO tombstone (entity, entity_id, profile_id, label, deleted_at, deleted_by)
    VALUES ('style_brick', OLD.id, OLD.profile_id, OLD.name, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device))
    ON CONFLICT (entity, entity_id) DO UPDATE SET
        profile_id = excluded.profile_id, label = excluded.label,
        deleted_at = excluded.deleted_at, deleted_by = excluded.deleted_by,
        restored_at = NULL, restored_by = NULL;
    DELETE FROM field_clock WHERE entity = 'style_brick' AND entity_id = OLD.id;
END;
