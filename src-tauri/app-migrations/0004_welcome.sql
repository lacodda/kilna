-- The first space of a new machine (v0.129, FR-16): kilna makes it so the
-- window has somewhere to stand, and the window welcomes the person into it -
-- what the space is made of, its name and look, a demo or not - instead of
-- asking only for a name. Set once, by the first start; cleared when the
-- person has chosen.
ALTER TABLE space ADD COLUMN welcome INTEGER NOT NULL DEFAULT 0;
