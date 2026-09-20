-- A seventh stop on the stage dial: "Final", for work that is out.
--
-- "Finished" and "out in the world" are two different claims, and the dial
-- only had room for the first: a song could be finished for a month before it
-- shipped, and nothing on the row said which. The stop is set by hand like
-- every other one - the stage stays a judgement (see 0023), and the fact of a
-- release is what the status already derives.
--
-- ## Why the numbers move
--
-- A stage is stored as the percentage of its stop, and `stageAt` reads a
-- number as the last stop it has reached. Seven evenly spaced stops are
-- 0/17/33/50/67/83/100, so the six that existed all shift down one place to
-- make room at the top.
--
-- Left alone, every work would keep its old number and be read against the
-- new grid: a work at 80 - "Polishing" - would land between 67 and 83 and
-- still read "Polishing", but a work at 100, set when 100 meant "Finished",
-- would silently become "Final" and claim to have been released. So the
-- stages already set are moved with the stops they were set at.
--
-- Highest first, so that no update collides with a value it is about to
-- write: moving 100 to 83 before 80 becomes 67 would otherwise sweep the
-- works at 80 along with it.
UPDATE work SET stage = 83 WHERE stage = 100;
UPDATE work SET stage = 67 WHERE stage = 80;
UPDATE work SET stage = 50 WHERE stage = 60;
UPDATE work SET stage = 33 WHERE stage = 40;
UPDATE work SET stage = 17 WHERE stage = 20;

-- The stops themselves live in the profile document, not in a table, so they
-- are not moved here: `regrade_stages` does it at open, putting a stop that
-- kept a shipped key on the number the shipped list now gives it while
-- leaving any word the owner renamed alone. This file moves the WORKS, which
-- is the part a migration can reach and the part that would otherwise lie.
