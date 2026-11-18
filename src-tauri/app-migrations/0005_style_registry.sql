-- The registries of style packs this machine reads (v0.131, FR-06): folders
-- and sites the person added, after the snapshot shipped with kilna, which
-- is always read and is not a row; and whether to read sites at all.
CREATE TABLE style_source (
    kind TEXT NOT NULL CHECK (kind IN ('folder', 'url')),
    location TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL
);

CREATE TABLE style_registry (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    offline INTEGER NOT NULL DEFAULT 0
);
