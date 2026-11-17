-- Types of space (v0.112, ADR 0079): a profile kept as what a new space is
-- made of.
--
-- A space is made from a profile kilna ships - Studio, Blog, Novel, Podcast,
-- or the empty one - and its vocabulary is then edited in it. A second channel
-- of the same craft used to start from the shipped words again, with every
-- axis, status and action the first one had grown to be reworded by hand. A
-- type is that vocabulary kept: the document of a space's profile as it stood
-- when it was saved, or one read from a file, offered beside the shipped ones
-- when a space is made.
--
-- Here, in the application's own database, and not in a space: a type is
-- what spaces are made of on this machine, and a space made from one holds
-- its own copy from then on - editing the type changes no space, and editing
-- the space changes no type.
CREATE TABLE space_type (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL CHECK (length(name) > 0),
    description TEXT,
    -- The shipped profile it grew from: `music`, `blog`, `blank`. A space made
    -- of the type is a space of this kind - it looks like one until given a
    -- look of its own - and its profile is carried forward as this kind's is,
    -- so an action shipped next month reaches it too.
    kind        TEXT NOT NULL CHECK (length(kind) > 0),
    -- The profile document, in the format it was written in; read through the
    -- same parser a space's stored copy is.
    config      TEXT NOT NULL,
    created_at  TEXT NOT NULL
) STRICT;
