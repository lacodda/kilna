-- The AI panel. Kept out of the first migration deliberately: the core loop
-- had to stand on its own, and this is the first change the migration
-- machinery actually carries.

-- One conversation, optionally about a particular work.
CREATE TABLE chat (
    id         TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profile (id) ON DELETE CASCADE,
    work_id    TEXT REFERENCES work (id) ON DELETE CASCADE,
    title      TEXT,
    -- Session id reported by the CLI, so a later turn can resume the same
    -- conversation instead of starting a fresh one.
    session_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

CREATE INDEX chat_profile ON chat (profile_id, updated_at DESC);
CREATE INDEX chat_work ON chat (work_id, updated_at DESC);

CREATE TABLE chat_message (
    id         TEXT PRIMARY KEY,
    chat_id    TEXT NOT NULL REFERENCES chat (id) ON DELETE CASCADE,
    role       TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system')),
    body       TEXT NOT NULL,
    -- Cost, duration and model reported by the CLI for this turn.
    meta       TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL
) STRICT;

CREATE INDEX chat_message_chat ON chat_message (chat_id, created_at);
