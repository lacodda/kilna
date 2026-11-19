-- What a term of the register is looked for in (v0.151, FR-20 20.2): the
-- sung text, a title, what a picture shows, what a sound is made of, or all
-- of them - and how many works of a month may carry it, how near two may
-- stand. Every term written before reads the sung text, as it always did.
ALTER TABLE term ADD COLUMN scope TEXT NOT NULL DEFAULT 'lyrics';
ALTER TABLE term ADD COLUMN max_per_month INTEGER;
ALTER TABLE term ADD COLUMN cooldown_days INTEGER;
