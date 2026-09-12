-- A chat remembers the action that opened it, and the version it is about.
--
-- Until now a task's chat knew its work and nothing else: which action
-- started it lived in memory, on the run, and the version it read was
-- whichever was current at the moment. Two things need more than that.
--
-- The action's method (ADR 0021) has to reach the model on every turn of
-- the chat, not only the first: a follow-up question in a critique chat is
-- still a critique. So the chat carries the action's key, and the run reads
-- the method through it each time it starts.
--
-- And an action started from the versions tab is started on the version
-- that is open there, not on whatever is current. What the answer proposes
-- - a score, a commentary - is about that version, so the chat carries it
-- too, and applying binds to it. Set to nothing when the version goes: the
-- chat stays, the fact of which version it was goes with the version.

ALTER TABLE chat ADD COLUMN action TEXT;
ALTER TABLE chat ADD COLUMN version_id TEXT REFERENCES work_version (id) ON DELETE SET NULL;
