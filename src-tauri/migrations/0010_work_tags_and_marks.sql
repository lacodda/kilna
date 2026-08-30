-- Two ways to mark a work by hand, beside the status the automation derives.
--
-- The status answers "where is this in the process", and it is worked out from
-- what happened (v0.20). Neither of these is that: a tag is the author's own
-- vocabulary for what a work *is*, a mark is a flag they raise about it right
-- now. Both are set by hand and neither is ever derived.
--
-- Columns rather than tables, following `note.tags` from 0001: a work carries
-- its own list, filtering goes through `json_each`, and nothing has to be
-- joined to read a card. Two arrays in one migration because they arrive
-- together in the header and would otherwise be two migrations apart for no
-- reason a user could see.

-- The author's own words for what a work is: "ironic", "warm", "winter".
--
-- Free text, not a fixed list: the vocabulary of a craft is not something the
-- app gets to decide, and the predecessor proved the point — its 194 tagged
-- songs used 272 distinct words, none of them from a menu. Completion comes
-- from what the workspace already holds, so the list converges without ever
-- being closed.
ALTER TABLE work ADD COLUMN tags TEXT NOT NULL DEFAULT '[]';

-- Flags the profile offers: "in progress", "needs a decision", "on fire".
--
-- Stored as keys into `profile.config.marks`, so the label, the icon and the
-- colour live where every other piece of craft vocabulary lives, and renaming
-- one does not touch a single work. A key the profile no longer defines is
-- simply not drawn — the same rule the statuses follow.
--
-- Kept apart from tags although both are arrays of strings the user sets: a
-- tag describes the work and stays with it, a mark is about this week and is
-- taken off. Mixing them would make "on fire" sort beside "winter" in
-- completion, and clearing the flags would clear the vocabulary with them.
ALTER TABLE work ADD COLUMN marks TEXT NOT NULL DEFAULT '[]';
