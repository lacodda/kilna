-- The journal: what happened, in order, in words the app can still translate.
--
-- The table declared in 0001 stored a finished sentence in `summary`. That was
-- wrong in a way only a second language makes visible: a sentence written while
-- the interface was English stays English forever, and the locale gate cannot
-- see it to check. So an entry stores what happened — an action key and its
-- values — and the sentence is built at the moment it is read.
--
-- 0001's table never had a writer, so there is nothing to carry over; it is
-- replaced rather than altered.
DROP TABLE journal;

CREATE TABLE journal (
    id         TEXT PRIMARY KEY,
    profile_id TEXT REFERENCES profile (id) ON DELETE CASCADE,
    -- The i18n key naming what happened: 'work.created', 'release.displaced'.
    -- The same key feeds the toast, so a line of history and the message shown
    -- at the time cannot drift apart.
    action     TEXT NOT NULL,
    -- Values the key interpolates: {"title": "Winter road"}. Titles are copied
    -- in on purpose — history has to still read correctly after the work it
    -- talks about is renamed or deleted.
    params     TEXT NOT NULL DEFAULT '{}',
    -- How loudly it asks to be noticed. 'info' is the ordinary record; 'warn'
    -- is something the person should look at. Only 'warn' is counted unread.
    level      TEXT NOT NULL DEFAULT 'info',
    -- What it happened to, so a card can show its own history. Deliberately not
    -- a foreign key: the entry outlives the row, which is most of the point.
    entity     TEXT,
    entity_id  TEXT,
    -- Set when two of the same thing should collapse into one line instead of
    -- filling the feed. A repeat updates the existing entry's timestamp and
    -- count rather than adding a row.
    dedupe_key TEXT,
    -- How many times it happened, counting the first.
    occurrences INTEGER NOT NULL DEFAULT 1,
    -- When it was last seen — a deduped entry moves forward.
    created_at TEXT NOT NULL,
    -- When the person saw it. NULL means unread, and unread entries are never
    -- swept: retention removes what has been read and grown old, never what was
    -- never looked at.
    read_at    TEXT
) STRICT;

CREATE INDEX journal_feed ON journal (profile_id, created_at DESC);
CREATE INDEX journal_entity ON journal (entity, entity_id, created_at DESC);

-- A dedupe key means one live entry per profile, which this enforces rather
-- than trusting every writer to check first.
CREATE UNIQUE INDEX journal_dedupe ON journal (profile_id, dedupe_key)
    WHERE dedupe_key IS NOT NULL;
