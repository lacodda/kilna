-- A style brick can come from the craft's starter set, and says more than a
-- description (v0.87).
--
-- Until now every brick was one the owner made. A shipped profile now carries a
-- starter set - image styles, typography, dressings, backgrounds - seeded into
-- the dictionary when the workspace opens (ADR 0048). A seeded brick is the
-- owner's from the moment it lands: they may edit it, drop it or delete it.
-- What the set needs from the row is to know, later, which of those happened.
--
-- `set_key` names the entry of the set the brick came from; NULL is a brick of
-- the owner's own. `set_digest` is the fingerprint of the entry as it was last
-- written into the row. The row's own fingerprint equal to it means nobody
-- touched the brick since - it reads "from the set", and a newer set may
-- rewrite it. Different means the owner changed it - it reads "changed", keeps
-- what they wrote, and offers "restore as in the set". A fingerprint rather
-- than a flag: an edit typed and then typed back is no edit at all.
ALTER TABLE style_brick ADD COLUMN set_key TEXT;
ALTER TABLE style_brick ADD COLUMN set_digest TEXT;

-- The name per language, as a shipped word carries it ({"en": .., "ru": ..}).
-- `name` stays the one word the dictionary is unique on (the English one for a
-- seeded brick); the window shows the word of its language while this is set.
-- Renaming a brick clears it: the name typed is the owner's, in one language.
ALTER TABLE style_brick ADD COLUMN label TEXT;

-- The family of an image style: a key of its type's `families` in the profile
-- document. A string for the reason `type_key` is one (0026).
ALTER TABLE style_brick ADD COLUMN family TEXT;

-- When to reach for the brick, for whoever picks bricks for a picture - the
-- idea generator reads it. Not the steer (`hint`): the steer is about how the
-- description is written, this is about when the brick is the right one.
ALTER TABLE style_brick ADD COLUMN when_to_use TEXT;

-- Colours, as a JSON array of `#RRGGBB`: the one colour of a background, the
-- palette an image style is shown by while it has no pictures.
ALTER TABLE style_brick ADD COLUMN colours TEXT NOT NULL DEFAULT '[]';

-- How a lettering brick is shown on its card: CSS declarations for a live
-- sample of the type. For the eye only - on a picture the lettering is drawn
-- by the generator from the description.
ALTER TABLE style_brick ADD COLUMN sample TEXT;

-- One set entry is one brick per workspace.
CREATE UNIQUE INDEX style_brick_set ON style_brick (profile_id, set_key) WHERE set_key IS NOT NULL;
