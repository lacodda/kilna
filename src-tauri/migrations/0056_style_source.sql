-- Where a brick of the style dictionary came from, when a pack of a
-- registry gave it (v0.131, FR-06): `pack:<id>`. A brick of the profile's
-- starter set or of the owner's own leaves it empty. With `set_key` - the
-- brick's key in the pack - and `set_digest` - the fingerprint of the words
-- the pack last gave it - the brick is a shipped entry like any other
-- (ADR 0087).
ALTER TABLE style_brick ADD COLUMN source TEXT;
