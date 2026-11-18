-- When a brick can go into a picture and what it brings to it (v0.134,
-- FR-07 7.2): {"needs": [...], "provides": [...], "conflicts": [...]}, as
-- JSON. Empty for a brick that goes anywhere.
ALTER TABLE style_brick ADD COLUMN contract TEXT;
