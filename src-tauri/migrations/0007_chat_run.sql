-- Runs of the assistant CLI.
--
-- Before this, a turn lived inside one command call: the panel asked, waited,
-- and got a message back. A run that takes minutes cannot be held that way —
-- navigating away would abandon it — so a run is now a row of its own,
-- belonging to the chat rather than to whoever asked.
--
-- The row is what survives a crash: a run left `running` when the application
-- died is swept to `broken` at the next start, because the process that was
-- doing the work is gone with it.

CREATE TABLE chat_run (
    id         TEXT PRIMARY KEY,
    chat_id    TEXT NOT NULL REFERENCES chat (id) ON DELETE CASCADE,
    -- What was sent, kept next to the run so a failed one can be read back.
    prompt     TEXT NOT NULL,
    state      TEXT NOT NULL CHECK (state IN ('running', 'done', 'failed', 'cancelled', 'broken')),
    -- Why it ended, when it ended badly.
    detail     TEXT,
    -- Every event the run produced, as a JSON array. This is the transcript
    -- the panel replays when it comes back to a chat mid-run.
    events     TEXT NOT NULL DEFAULT '[]',
    started_at TEXT NOT NULL,
    ended_at   TEXT
) STRICT;

CREATE INDEX chat_run_chat ON chat_run (chat_id, started_at DESC);
CREATE INDEX chat_run_state ON chat_run (state);
