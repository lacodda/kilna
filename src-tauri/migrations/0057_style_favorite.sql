-- A style the owner reaches for first (v0.132, FR-05 5.2): starred on its
-- card, lifted in the pickers, narrowed to by a chip. Not part of what the
-- style says, so a starred shipped style still reads untouched.
ALTER TABLE style_brick ADD COLUMN favorite INTEGER NOT NULL DEFAULT 0 CHECK (favorite IN (0, 1));
