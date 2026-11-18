-- What a space was given of a shipped profile, entry by entry (v0.125,
-- ADR 0087): for each entry of a list kilna goes on shipping - an action, a
-- kind's status, a stop, a mark, a field, a kind of note or its section, a
-- word of the vibe - the fingerprint of the version last given to it, and
-- the shipped version its owner chose to keep theirs over. Beside the
-- profile document rather than in it: the document is the owner's to read,
-- edit and pass on. A row with no entry by its key in the document is an
-- entry its owner deleted, and the row is what keeps it from coming back.
CREATE TABLE shipped_entry (
    profile_id TEXT NOT NULL REFERENCES profile (id) ON DELETE CASCADE,
    -- Where the entry comes from: `profile:<key>`, and `pack:<id>` once packs
    -- of styles arrive.
    origin TEXT NOT NULL,
    -- The list it is in, by the keys on the way: `prompts`,
    -- `work_kinds/song/statuses`, `note_kinds/character/sections`.
    collection TEXT NOT NULL,
    source_key TEXT NOT NULL,
    -- Empty for an entry that was already its owner's when first met.
    digest TEXT NOT NULL,
    kept TEXT,
    PRIMARY KEY (profile_id, collection, source_key)
);
