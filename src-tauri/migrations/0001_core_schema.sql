-- Core schema. One fixed shape for every craft; the craft lives in `profile.config`.
-- See docs/adr/0001-one-schema-with-profile-configuration.md and 0002-local-first-storage.md.

-- A craft scenario: vocabulary, axes, tiers, statuses, version roles, meta fields.
CREATE TABLE profile (
    id          TEXT PRIMARY KEY,
    key         TEXT NOT NULL UNIQUE,
    name        TEXT NOT NULL,
    description TEXT,
    -- Full scenario document; keys inside are stable, labels are free to change.
    config      TEXT NOT NULL,
    -- Exactly one profile is active; enforced by a partial unique index below.
    is_active   INTEGER NOT NULL DEFAULT 0 CHECK (is_active IN (0, 1)),
    is_builtin  INTEGER NOT NULL DEFAULT 0 CHECK (is_builtin IN (0, 1)),
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
) STRICT;

CREATE UNIQUE INDEX profile_single_active ON profile (is_active) WHERE is_active = 1;

-- An album, a book, a season. One level deep: a collection never contains a collection.
CREATE TABLE collection (
    id          TEXT PRIMARY KEY,
    profile_id  TEXT NOT NULL REFERENCES profile (id) ON DELETE CASCADE,
    kind        TEXT NOT NULL,
    title       TEXT NOT NULL,
    description TEXT,
    position    INTEGER NOT NULL DEFAULT 0,
    meta        TEXT NOT NULL DEFAULT '{}',
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
) STRICT;

CREATE INDEX collection_profile ON collection (profile_id, position);

-- The thing being made: a song, a chapter, an episode.
CREATE TABLE work (
    id                 TEXT PRIMARY KEY,
    profile_id         TEXT NOT NULL REFERENCES profile (id) ON DELETE CASCADE,
    collection_id      TEXT REFERENCES collection (id) ON DELETE SET NULL,
    kind               TEXT NOT NULL,
    title              TEXT NOT NULL,
    status             TEXT NOT NULL,
    -- Craft-specific fields, keyed by `work_meta_fields` in the profile config.
    meta               TEXT NOT NULL DEFAULT '{}',
    -- The version that represents the work right now.
    current_version_id TEXT REFERENCES work_version (id) ON DELETE SET NULL DEFERRABLE INITIALLY DEFERRED,
    position           INTEGER NOT NULL DEFAULT 0,
    created_at         TEXT NOT NULL,
    updated_at         TEXT NOT NULL
) STRICT;

CREATE INDEX work_profile_status ON work (profile_id, status);
CREATE INDEX work_collection ON work (collection_id, position);

-- A draft kept whole. Bodies are stored in full, never as a diff chain.
CREATE TABLE work_version (
    id         TEXT PRIMARY KEY,
    work_id    TEXT NOT NULL REFERENCES work (id) ON DELETE CASCADE,
    -- Role from the profile: lyrics, style, outline, script…
    role       TEXT NOT NULL,
    -- Monotonic per (work, role).
    revision   INTEGER NOT NULL,
    label      TEXT,
    body       TEXT NOT NULL,
    meta       TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    UNIQUE (work_id, role, revision)
) STRICT;

CREATE INDEX work_version_work ON work_version (work_id, role, revision DESC);

-- A score is a snapshot pinned to a version, never an overwrite of the work.
CREATE TABLE work_score (
    id          TEXT PRIMARY KEY,
    work_id     TEXT NOT NULL REFERENCES work (id) ON DELETE CASCADE,
    version_id  TEXT REFERENCES work_version (id) ON DELETE SET NULL,
    -- Axis keys are profile strings, not foreign keys: {"hook": 8, "text": 7}.
    axes        TEXT NOT NULL,
    total       REAL NOT NULL,
    tier        TEXT,
    note        TEXT,
    scored_at   TEXT NOT NULL
) STRICT;

CREATE INDEX work_score_work ON work_score (work_id, scored_at DESC);

-- What ships, where, when. The predecessor's three tables collapse into this one.
CREATE TABLE release (
    id           TEXT PRIMARY KEY,
    work_id      TEXT NOT NULL REFERENCES work (id) ON DELETE CASCADE,
    kind         TEXT NOT NULL,
    status       TEXT NOT NULL,
    title        TEXT,
    -- Calendar slot; NULL means unscheduled.
    scheduled_at TEXT,
    released_at  TEXT,
    url          TEXT,
    meta         TEXT NOT NULL DEFAULT '{}',
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL
) STRICT;

CREATE INDEX release_schedule ON release (scheduled_at);
CREATE INDEX release_work ON release (work_id);

-- Ideas, lore, reference — one type with tags rather than four subsystems.
CREATE TABLE note (
    id         TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profile (id) ON DELETE CASCADE,
    work_id    TEXT REFERENCES work (id) ON DELETE CASCADE,
    kind       TEXT NOT NULL DEFAULT 'note',
    title      TEXT,
    body       TEXT NOT NULL,
    -- JSON array of strings.
    tags       TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

CREATE INDEX note_profile ON note (profile_id, updated_at DESC);
CREATE INDEX note_work ON note (work_id);

-- Media lives on disk; the database holds the path.
CREATE TABLE asset (
    id         TEXT PRIMARY KEY,
    work_id    TEXT REFERENCES work (id) ON DELETE CASCADE,
    release_id TEXT REFERENCES release (id) ON DELETE CASCADE,
    kind       TEXT NOT NULL,
    path       TEXT NOT NULL,
    label      TEXT,
    meta       TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL
) STRICT;

CREATE INDEX asset_work ON asset (work_id);
CREATE INDEX asset_release ON asset (release_id);

-- What happened, in order. Append-only.
CREATE TABLE journal (
    id         TEXT PRIMARY KEY,
    profile_id TEXT REFERENCES profile (id) ON DELETE SET NULL,
    work_id    TEXT REFERENCES work (id) ON DELETE SET NULL,
    action     TEXT NOT NULL,
    summary    TEXT NOT NULL,
    detail     TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL
) STRICT;

CREATE INDEX journal_created ON journal (created_at DESC);
CREATE INDEX journal_work ON journal (work_id, created_at DESC);
