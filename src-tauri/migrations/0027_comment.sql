-- Comments from the audience: what people wrote under a released work, and
-- the reply to it.
--
-- ## Why a table of its own, and not a note with a kind
--
-- A comment has things a note does not: where it was written (a channel),
-- who wrote it, when they wrote it, the reply, and where that reply stands.
-- Carried by a note they would be fields in a JSON column that only one kind
-- of note has - a second schema hidden inside the first, which is the thing
-- ADR 0013 draws the line at: a fact about a row is a column.
--
-- ## The channel is a word, not a row
--
-- The predecessor filed comments under a channel slug, and four channels out
-- of five were not rows of its database: a comment comes from wherever the
-- work went out, and nobody keeps a table of those. So the channel is the
-- text the person typed, and the list of channels is the list of words
-- already used. A table of channels would be a form to fill in before the
-- first comment could be kept, for a list the comments already are.
--
-- ## The work is a key, not a title
--
-- The predecessor matched a comment to its song by comparing normalised
-- titles, which broke on every rename and every pair of quotes. Here it is
-- the work's id. A comment about no work in particular - "love the channel" -
-- has none, which is allowed.
--
-- ## Three states, and no fourth
--
-- `open` is waiting for the person, `posted` is answered where it was
-- written, `archived` needs nothing. "A reply is drafted but not sent" is
-- read off `reply` being non-empty on an open comment, not stored as a state
-- of its own: stored, it could say "drafted" over an empty reply, or "open"
-- over a finished one. A reply is always posted by hand - kilna does not
-- speak for anyone on their channel - so `posted` is set by the person.
--
-- ## No screenshot column
--
-- A comment usually arrives as a screenshot. The predecessor kept the file
-- beside the row until the text was typed in, with a placeholder body, and in
-- the end not one of its 121 comments still held a screenshot: the picture is
-- a way in, not something to keep. Here the picture is read by the assistant
-- into a proposal, and the comment exists only once the person has kept that
-- proposal - so there is never a row with no text and a file standing in.

CREATE TABLE comment (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profile (id) ON DELETE CASCADE,
    -- Where it was written, as the person names it.
    channel TEXT NOT NULL,
    -- The work it is about. Goes with the work to the trash and comes back
    -- with it (see `trash::cascade`).
    work_id TEXT REFERENCES work (id) ON DELETE CASCADE,
    -- Who wrote it, when that is known.
    author TEXT,
    body TEXT NOT NULL,
    -- The answer, drafted here and posted by hand.
    reply TEXT,
    state TEXT NOT NULL DEFAULT 'open',
    -- The day the viewer wrote it, YYYY-MM-DD. A date, not an instant: a
    -- screenshot says "3 weeks ago", never a time zone.
    commented_on TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (trim(channel) <> ''),
    CHECK (trim(body) <> ''),
    CHECK (state IN ('open', 'posted', 'archived')),
    CHECK (commented_on IS NULL OR commented_on GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]')
);

-- The inbox: one profile's comments by where they stand, newest first.
CREATE INDEX comment_inbox ON comment (profile_id, state, created_at DESC);
-- A work's comments, for its card and its counter.
CREATE INDEX comment_work ON comment (work_id);

-- Tombstones, the shape 0011 gives every table a person's work lives in.
CREATE TRIGGER comment_tombstone_on_delete AFTER DELETE ON comment
BEGIN
    INSERT INTO tombstone (entity, entity_id, profile_id, label, deleted_at, deleted_by)
    VALUES ('comment', OLD.id, OLD.profile_id, substr(OLD.body, 1, 80), strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device))
    ON CONFLICT (entity, entity_id) DO UPDATE SET
        profile_id = excluded.profile_id, label = excluded.label,
        deleted_at = excluded.deleted_at, deleted_by = excluded.deleted_by,
        restored_at = NULL, restored_by = NULL;
    DELETE FROM field_clock WHERE entity = 'comment' AND entity_id = OLD.id;
END;

CREATE TRIGGER comment_tombstone_on_insert AFTER INSERT ON comment
BEGIN
    UPDATE tombstone SET restored_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), restored_by = (SELECT id FROM device)
    WHERE entity = 'comment' AND entity_id = NEW.id AND restored_at IS NULL;
END;

-- Field clocks, so a comment answered on one machine and archived on another
-- settles the way every other edit settles (0011). One statement per column:
-- a column left out here is invisible to synchronisation (the lesson of 0023).
CREATE TRIGGER comment_clock_on_update AFTER UPDATE ON comment
BEGIN
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'comment', NEW.id, 'channel', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.channel IS NOT NEW.channel
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'comment', NEW.id, 'work_id', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.work_id IS NOT NEW.work_id
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'comment', NEW.id, 'author', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.author IS NOT NEW.author
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'comment', NEW.id, 'body', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.body IS NOT NEW.body
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'comment', NEW.id, 'reply', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.reply IS NOT NEW.reply
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'comment', NEW.id, 'state', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.state IS NOT NEW.state
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
    INSERT INTO field_clock (entity, entity_id, field, changed_at, device_id)
    SELECT 'comment', NEW.id, 'commented_on', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), (SELECT id FROM device) WHERE OLD.commented_on IS NOT NEW.commented_on
    ON CONFLICT (entity, entity_id, field) DO UPDATE SET
        changed_at = excluded.changed_at, device_id = excluded.device_id;
END;

-- What the search finds a comment by: its words, its reply, who wrote it and
-- where. "The one who asked about the bridge" is a search a person makes.
CREATE TRIGGER search_comment_on_insert AFTER INSERT ON comment
BEGIN
    INSERT INTO search_index (entity, entity_id, profile_id, work_id, body)
    VALUES ('comment', NEW.id, NEW.profile_id, NEW.work_id,
        NEW.body || ' ' || coalesce(NEW.reply, '') || ' ' ||
        coalesce(NEW.author, '') || ' ' || NEW.channel);
END;

CREATE TRIGGER search_comment_on_update AFTER UPDATE ON comment
WHEN OLD.body IS NOT NEW.body OR OLD.reply IS NOT NEW.reply OR OLD.author IS NOT NEW.author
     OR OLD.channel IS NOT NEW.channel OR OLD.work_id IS NOT NEW.work_id
BEGIN
    DELETE FROM search_index WHERE entity = 'comment' AND entity_id = NEW.id;
    INSERT INTO search_index (entity, entity_id, profile_id, work_id, body)
    VALUES ('comment', NEW.id, NEW.profile_id, NEW.work_id,
        NEW.body || ' ' || coalesce(NEW.reply, '') || ' ' ||
        coalesce(NEW.author, '') || ' ' || NEW.channel);
END;

CREATE TRIGGER search_comment_on_delete AFTER DELETE ON comment
BEGIN
    DELETE FROM search_index WHERE entity = 'comment' AND entity_id = OLD.id;
END;
