-- The focus board: what the person decided about what the workspace noticed.
--
-- Findings (v0.33) are derived, not stored — they are recomputed from the
-- catalogue and the calendar on every read, and a finding disappears on its own
-- the moment its complaint stops being true. Nothing here stores a finding.
-- What is stored is the two things derivation cannot know: a complaint the
-- person has already answered, and a line they wrote themselves.

-- A dismissed complaint.
--
-- Dismissing is not "hide this work" and not "hide this kind" — it is "I have
-- heard *this*". The complaint string carries what was actually said, so a
-- changed complaint about the same work is news again: 'stale-draft:1' hidden
-- in January says nothing about 'stale-draft:4' in April. That is why v0.33
-- rounded the unstable ones to months rather than days — a complaint that
-- changed every morning would come back every morning.
--
-- No foreign key to `work`: the row is a decision, not a fact about a work, and
-- a dismissal for a work that has since gone is simply never matched again.
-- Sweeping them is `dismissal_sweep` below rather than a cascade.
CREATE TABLE focus_dismissal (
    id           TEXT PRIMARY KEY,
    profile_id   TEXT NOT NULL REFERENCES profile (id) ON DELETE CASCADE,
    -- The finding kind, matching `FindingKind` on the front end.
    kind         TEXT NOT NULL,
    work_id      TEXT NOT NULL,
    -- The exact complaint that was dismissed. Stored whole rather than parsed:
    -- the back end has no opinion about what is inside it, which is what lets
    -- a new finding kind ship without a migration.
    complaint    TEXT NOT NULL,
    dismissed_at TEXT NOT NULL
) STRICT;

-- The board asks "is this one dismissed?" for every finding it draws, so the
-- lookup is by exactly the three things that identify a complaint.
CREATE UNIQUE INDEX focus_dismissal_complaint
    ON focus_dismissal (profile_id, kind, work_id, complaint);

-- A note the person put on the board themselves.
--
-- Deliberately not a `note` row. A `note` is content about a work — an idea, a
-- lyric, a piece of reference — and it outlives the board. This is a line on a
-- surface: "ask the label about the artwork", true this week and gone the next.
-- Filing it as a note would put it in the notes screen, in search and in the
-- work's card, where it is noise.
CREATE TABLE focus_note (
    id         TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profile (id) ON DELETE CASCADE,
    body       TEXT NOT NULL,
    -- Optional subject. A note about a work opens that work when clicked; one
    -- without stands on its own. Not a foreign key for the same reason the
    -- journal's is not: the line has to still read after the work is deleted.
    work_id    TEXT,
    -- Where it sits among the other notes. Sparse on purpose — reordering
    -- rewrites the moved row and leaves its neighbours alone.
    position   INTEGER NOT NULL,
    -- Set when the person put it at the top and wants it kept there. Notes are
    -- the only thing on the board a pin makes sense for: a finding leaves by
    -- itself when its complaint stops being true, so pinning one would only
    -- promise to keep something that is about to vanish anyway.
    pinned_at  TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;

CREATE INDEX focus_note_board ON focus_note (profile_id, position);
