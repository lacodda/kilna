-- A pinned status: the one place the person overrules the automation.
--
-- Status used to be written from wherever a fact changed — scoring set one
-- value, scheduling another, the card a third — and it drifted. From here the
-- automation derives it from what actually happened (a release exists, a score
-- exists, a draft exists) and this column is how a person says "leave it".
--
-- NULL means the automation owns the status. A timestamp means someone set it
-- by hand, and the automation steps over the work entirely.
ALTER TABLE work ADD COLUMN status_pinned_at TEXT;
